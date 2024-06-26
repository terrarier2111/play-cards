use std::collections::HashMap;

use thin_vec::{thin_vec, ThinVec};

use crate::{
    ast::AstNode,
    parser::Stmt,
    rt::{Ordering, RtRef, RtType},
};

#[derive(Debug)]
#[repr(u8)]
pub enum ByteCode {
    Push {
        val: RtRef,
    },
    Pop {
        /// the offset describes how many value should be skipped starting from the most recent element
        /// when looking for an element to pop from the stack
        offset: u8, // offsets other than 0 and 1 are unsupported
    },
    Mov {
        src_idx: UHalf,
        dst_idx: UHalf,
    },
    Call {
        fn_idx: u8,
        push_val: bool,
        arg_indices: ThinVec<UHalf>,
    },
    Add {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
    },
    Sub {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
    },
    Mul {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
    },
    Div {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
    },
    Mod {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
    },
    And {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
    },
    Or {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
    },
    Jump {
        relative_off: isize,
    },
    JumpCond {
        relative_off: isize,
        /// the condition is stored on the stack at this idx
        arg_idx: UHalf,
    },
    Compare {
        arg1_idx: UHalf,
        arg2_idx: UHalf,
        expected: Ordering,
    },
    Return {
        has_val: bool,
    },
    CallLocal {
        relative_off: isize,
    },
}

#[cfg(target_pointer_width = "64")]
pub type UHalf = u32;
#[cfg(target_pointer_width = "32")]
pub type UHalf = u16;
#[cfg(target_pointer_width = "16")]
pub type UHalf = u8;

pub struct Function {
    pub params: &'static [RtType],
    pub var_len: bool,
    pub name: &'static str,
    pub call: fn(Vec<RtRef>) -> Option<RtRef>,
}

struct Scope {
    vars: Vec<String>,
}

struct ResolvableCall {
    args: usize,
    name: String,
    location_idx: usize,
}

struct Translator<'a> {
    code: Vec<ByteCode>,
    internal_fns: HashMap<String, InternalFn>,
    local_fns: &'a HashMap<String, bool>,
    fns: &'a Vec<Function>,
    stack_idx: usize,
    vars: HashMap<String, Vec<usize>>,
    call_resolution: Vec<ResolvableCall>,
}

impl<'a> Translator<'a> {
    fn translate_internal(&mut self, stmts: &Vec<Stmt>) {
        let mut curr_scope = Scope { vars: vec![] };
        let initial_stack_idx = self.stack_idx;
        for stmt in stmts {
            match stmt {
                Stmt::DefineVar {
                    name,
                    val,
                    reassign,
                } => {
                    let mut pops = 0;
                    let var_idx = self.translate_node(val, &mut pops);
                    if *reassign {
                        let indices = self.vars.get(name).unwrap();
                        let idx = indices.last().unwrap();
                        self.code.push(ByteCode::Mov {
                            dst_idx: *idx as UHalf,
                            src_idx: var_idx as UHalf,
                        });
                        for _ in 0..pops {
                            self.code.push(ByteCode::Pop { offset: 0 });
                        }
                        self.stack_idx -= pops;
                    } else {
                        curr_scope.vars.push(name.clone());
                        self.vars.entry(name.clone()).or_default().push(var_idx);
                    }
                }
                Stmt::CallFunc { name, args } => {
                    let fn_idx = self.resolve_fn_idx(name);

                    // FIXME: check arg types

                    // if the argument count doesn't match and if the function isn't of variable length
                    // or if the required arguments of the variable length function aren't present,
                    // fail calling and inform the user
                    if (!self.fns[fn_idx].var_len || self.fns[fn_idx].params.len() > args.len())
                        && self.fns[fn_idx].params.len() != args.len()
                    {
                        panic!(
                            "Function arg count mismatch (\"{}\")",
                            self.fns[fn_idx].name
                        );
                    }

                    let mut pops = 0;
                    let mut indices = thin_vec![];
                    for arg in args {
                        indices.push(self.translate_node(&arg, &mut pops) as UHalf);
                    }
                    self.code.push(ByteCode::Call {
                        fn_idx: fn_idx as u8,
                        push_val: false,
                        arg_indices: indices,
                    });
                    for _ in 0..pops {
                        self.code.push(ByteCode::Pop { offset: 0 });
                    }
                    self.stack_idx -= pops;
                }
                Stmt::Loop { stmts, condition } => {
                    // this loop logic works by jumping (at the beginning of the loop) to the condition which we shall put at the end of the loop
                    // and only ever jump up if the statement is true
                    let loop_start_len = self.code.len();
                    let mut pops = 0;
                    self.translate_internal(stmts);
                    let body_size = self.code.len() - loop_start_len;
                    println!("body size: {}", body_size);

                    // this is the argument for the condition which decides whether to continue with the loop
                    let arg_idx = self.translate_node(&condition, &mut pops);

                    // cleanup for when we are in the loop
                    for _ in 0..pops {
                        self.code
                            .insert(loop_start_len, ByteCode::Pop { offset: 0 });
                    }

                    // jump to the condition right at the start
                    self.code.insert(
                        loop_start_len,
                        ByteCode::Jump {
                            relative_off: (body_size + pops) as isize + 1,
                        },
                    );

                    // this includes the normal body size and all the additional code we generated for loop maintenance
                    // the + 1 if from the unconditional Jump we use to go back to the condition at the end of the loop
                    let full_body_size = self.code.len() - loop_start_len;

                    // skip the body if the inverse condition turns out to be true
                    self.code.push(ByteCode::JumpCond {
                        relative_off: -(full_body_size as isize - 1),
                        arg_idx: arg_idx as UHalf,
                    });
                    // cleanup for when we exit the loop
                    for _ in 0..pops {
                        self.code.push(ByteCode::Pop { offset: 0 });
                    }
                    self.stack_idx -= 1;
                }
                Stmt::Conditional { seq, fallback } => {
                    let mut jump_indices = vec![];
                    for (cond, stmts) in seq.iter() {
                        let mut pops = 0;
                        let cond_val_idx = self.translate_node(&cond, &mut pops);
                        let cond_idx = self.code.len();

                        let prev_code_size = self.code.len();
                        // cleanup condition data, if taken
                        for _ in 0..pops {
                            self.code.push(ByteCode::Pop { offset: 1 });
                        }
                        self.stack_idx -= pops;
                        self.translate_internal(stmts);
                        let code_size = self.code.len() - prev_code_size;
                        self.code.insert(
                            cond_idx,
                            ByteCode::JumpCond {
                                relative_off: code_size as isize,
                                arg_idx: cond_val_idx as UHalf,
                            },
                        );
                        // here, a unconditional jump to the end of the if statement will be inserted to skip any other conditional checks
                        // which ensures we are only ever taking a single path, not 2 or more
                        jump_indices.push(self.code.len());
                        // cleanup condition data, if not taken
                        for _ in 0..pops {
                            self.code.push(ByteCode::Pop { offset: 1 });
                        }
                    }
                    // insert the fallback (if present)
                    self.translate_internal(fallback);
                    // insert the jumps to the end of the if-(else) construct to ensure only 1 branch is ever taken
                    for idx in jump_indices.iter().rev() {
                        let end = self.code.len();
                        let off = end - *idx;
                        self.code.push(ByteCode::Jump {
                            relative_off: off as isize,
                        });
                    }
                }
                Stmt::DefineFn { name, args, stmts } => {
                    let code = translate_unit(stmts, self.fns, self.local_fns);
                    if !code.0.fns.is_empty() {
                        panic!("Nested function definitions are disallowed");
                    }
                    let internal_fn_idx = self.internal_fns.len();
                    self.internal_fns.insert(name.clone(), InternalFn {
                        code: code.0.main,
                        params: args.clone(),
                        call_resolution: code.1,
                        func_idx: internal_fn_idx,
                    });
                },
                Stmt::Return { val } => {
                    if let Some(val) = val {
                        let mut _pops = 0;
                        self.translate_node(val, &mut _pops);
                    }
                    self.code.push(ByteCode::Return { has_val: val.is_some() });
                    // discard all remaining code as it won't ever be executed and thus can be considered dead code
                    break;
                },
            }
        }
        for var in curr_scope.vars {
            // this doesn't necessarily delete all definitions of a certain variable as in case there are multiple variables with the same name,
            // it would come to collisions (if they are in different scopes)
            self.vars.get_mut(&var).unwrap().pop();
        }
        let stack_delta = self.stack_idx - initial_stack_idx;
        for _ in 0..stack_delta {
            self.code.push(ByteCode::Pop { offset: 0 });
        }
        self.stack_idx = initial_stack_idx;
    }

    /// returns the corresponding stack index
    fn translate_node(&mut self, node: &AstNode, pops: &mut usize) -> usize {
        match node {
            AstNode::CallFunc { name, params } => {
                if self.local_fns.contains_key(name) {
                    let mut call_pops = 0;
                    let mut indices = thin_vec![];
                    for param in params {
                        indices.push(self.translate_node(param, &mut call_pops) as UHalf);
                    }

                    // push a placeholder instruction to be replaced later on
                    self.code.push(ByteCode::CallLocal {
                        relative_off: 0,
                    }); // FIXME: should we push val?

                    let offset = if *self.local_fns.get(name).unwrap() { 1 } else { 0 };
                    for _ in 0..call_pops {
                        self.code.push(ByteCode::Pop { offset });
                    }

                    *pops += 1;
                    self.stack_idx += 1;

                    self.stack_idx -= call_pops;

                    return self.stack_idx - 1;
                }
                let func_idx = self.resolve_fn_idx(name);

                let mut call_pops = 0;
                let mut indices = thin_vec![];
                for param in params {
                    indices.push(self.translate_node(param, &mut call_pops) as UHalf);
                }

                self.code.push(ByteCode::Call {
                    fn_idx: func_idx as u8,
                    push_val: true,
                    arg_indices: indices,
                }); // FIXME: should we push val?

                for _ in 0..call_pops {
                    self.code.push(ByteCode::Pop { offset: 1 });
                }

                *pops += 1;
                self.stack_idx += 1;

                self.stack_idx -= call_pops;

                self.stack_idx - 1
            }
            AstNode::BinOp { lhs, rhs, op } => {
                let mut local_pops = 0;
                let idx1 = self.translate_node(lhs, &mut local_pops);
                let idx2 = self.translate_node(rhs, &mut local_pops);
                match op {
                    crate::ast::BinOpKind::Add => {
                        self.code.push(ByteCode::Add {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                        });
                    }
                    crate::ast::BinOpKind::Sub => {
                        self.code.push(ByteCode::Sub {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                        });
                    }
                    crate::ast::BinOpKind::Mul => {
                        self.code.push(ByteCode::Mul {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                        });
                    }
                    crate::ast::BinOpKind::Div => {
                        self.code.push(ByteCode::Div {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                        });
                    }
                    crate::ast::BinOpKind::Mod => {
                        self.code.push(ByteCode::Mod {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                        });
                    }
                    crate::ast::BinOpKind::And => {
                        self.code.push(ByteCode::And {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                        });
                    }
                    crate::ast::BinOpKind::Or => {
                        self.code.push(ByteCode::Or {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                        });
                    }
                    crate::ast::BinOpKind::Eq => {
                        self.code.push(ByteCode::Compare {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                            expected: Ordering::Equal,
                        });
                    }
                    crate::ast::BinOpKind::Ne => {
                        self.code.push(ByteCode::Compare {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                            expected: Ordering::NotEqual,
                        });
                    }
                    crate::ast::BinOpKind::Gt => {
                        self.code.push(ByteCode::Compare {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                            expected: Ordering::Greater,
                        });
                    }
                    crate::ast::BinOpKind::Lt => {
                        self.code.push(ByteCode::Compare {
                            arg1_idx: idx1 as UHalf,
                            arg2_idx: idx2 as UHalf,
                            expected: Ordering::Less,
                        });
                    }
                    crate::ast::BinOpKind::Ge => {
                        self.code.push(ByteCode::Compare {
                            arg1_idx: idx2 as UHalf,
                            arg2_idx: idx1 as UHalf,
                            expected: Ordering::Less,
                        });
                    }
                    crate::ast::BinOpKind::Le => {
                        self.code.push(ByteCode::Compare {
                            arg1_idx: idx2 as UHalf,
                            arg2_idx: idx1 as UHalf,
                            expected: Ordering::Greater,
                        });
                    }
                }
                for _ in 0..local_pops {
                    self.code.push(ByteCode::Pop { offset: 1 });
                }
                self.stack_idx -= local_pops;

                *pops += 1;
                self.stack_idx += 1;
                self.stack_idx - 1
            }
            AstNode::Val(val) => {
                self.code.push(ByteCode::Push { val: *val });
                *pops += 1;
                self.stack_idx += 1;
                self.stack_idx - 1
            }
            AstNode::Var { name } => {
                if let Some(val) = self.vars.get(name) {
                    return *val.last().unwrap();
                }
                if let Some(val) = self.internal_fns.get(name) {
                    self.code.push(ByteCode::Push { val: RtRef::function(val.func_idx) });
                    *pops += 1;
                    self.stack_idx += 1;
                    return self.stack_idx - 1;
                }

                panic!("No variable or function named \"{}\"", name);
            },
            AstNode::UnaryOp { val, op } => match *op {
                crate::ast::UnaryOpKind::Not => {
                    todo!()
                }
            },
        }
    }

    // FIXME: handle missing functions
    fn resolve_fn_idx(&self, fn_name: &String) -> usize {
        let fn_idx = self
            .fns
            .iter()
            .enumerate()
            .find(|(_, val)| &val.name == fn_name)
            .expect(format!("Function \"{fn_name}\" doesn't exist").as_str())
            .0;
        fn_idx
    }

    fn optimize(&mut self) {
        let mut optimized = false;
        while !optimized {
            optimized = true;
            // try eliminating push/pop(0) sequences
            for i in 1..self.code.len() {
                if let ByteCode::Push { .. } = self.code[i - 1] {
                    if let ByteCode::Pop { offset: 0 } = self.code[i] {
                        // fixup jumps
                        for j in 0..self.code.len() {
                            if let ByteCode::Jump { relative_off }
                            | ByteCode::JumpCond { relative_off, .. } = &mut self.code[j]
                            {
                                let other = ((j as isize) + *relative_off) as usize;
                                let range = j.min(other)..(j.max(other));
                                let mut containing = 0;
                                if range.contains(&i) {
                                    containing += 1;
                                }
                                if range.contains(&(i - 1)) {
                                    containing += 1;
                                }
                                if containing != 0 {
                                    if relative_off.is_negative() {
                                        *relative_off += containing;
                                    } else {
                                        *relative_off -= containing;
                                    }
                                }
                            }
                        }
                        self.code.remove(i);
                        self.code.remove(i - 1);
                        optimized = false;
                        // FIXME: deallocate if necessary

                        // continue optimization cycle
                        break;
                    }
                }
            }
        }
    }
}

struct TranslationOutput {
    main: Vec<ByteCode>,
    fns: HashMap<String, InternalFn>,
    
}

struct InternalFn {
    func_idx: usize,
    code: Vec<ByteCode>,
    params: Vec<String>,
    call_resolution: Vec<ResolvableCall>,
}

fn translate_unit(stmts: &Vec<Stmt>, fns: &Vec<Function>, local_fns: &HashMap<String, bool>) -> (TranslationOutput, Vec<ResolvableCall>) {
    let mut translator = Translator {
        code: vec![],
        fns,
        stack_idx: 0,
        vars: HashMap::new(),
        internal_fns: HashMap::new(),
        call_resolution: vec![],
        local_fns,
    };
    translator.translate_internal(stmts);
    translator.optimize();
    (TranslationOutput {
        main: translator.code,
        fns: translator.internal_fns,
    }, translator.call_resolution)
}

struct IntermediateFn {
    args: usize,
    offset: usize,
    call_res: Vec<ResolvableCall>,
}

fn discover_fn_defs(stmts: &Vec<Stmt>) -> anyhow::Result<HashMap<String, bool>> {
    let mut defs = HashMap::new();
    for stmt in stmts {
        if let Stmt::DefineFn { name, stmts, .. } = stmt {
            let mut has_val = None;
            for stmt in stmts {
                let val = if let Stmt::Return { val: Some(_) } = stmt {
                    true
                } else {
                    false
                };
                if let Some(curr_val) = has_val {
                    if curr_val != val {
                        panic!("A function may only always or never return a value");
                    }
                } else {
                    has_val = Some(val);
                }
            }
            defs.insert(name.clone(), has_val.unwrap_or(false));
        }
    }
    Ok(defs)
}

pub fn translate(stmts: &Vec<Stmt>, fns: &Vec<Function>) -> Vec<ByteCode> {
    let fn_defs = discover_fn_defs(stmts).unwrap();
    let res = translate_unit(stmts, fns, &fn_defs);

    // resolve addresses of local functions at call sites
    let mut bc = res.0.main;
    let mut fn_stack = vec![];
    let mut fn_lookup = HashMap::new();
    for fun in res.0.fns {
        fn_stack.push(IntermediateFn {
            args: fun.1.params.len(),
            offset: bc.len(),
            call_res: fun.1.call_resolution,
        });
        fn_lookup.insert(fun.0.clone(), fn_stack.len() - 1);
        bc.extend(fun.1.code);
    }
    for i in (0..fn_stack.len()).rev() {
        for res in &fn_stack[i].call_res {
            let fun = &fn_stack[*fn_lookup.get(&res.name).expect("Function call to unknown function")];
            if fun.args != res.args {
                panic!("Argument count mismatch");
            }
            bc[fn_stack[i].offset + res.location_idx] = ByteCode::CallLocal { relative_off: (fun.offset - res.location_idx) as isize };
        }
    }
    // FIXME: handle functions with and without return value properly!
    for res in res.1 {
        let fun = &fn_stack[*fn_lookup.get(&res.name).expect("Function call to unknown function")];
        if fun.args != res.args {
            panic!("Argument count mismatch");
        }
        bc[res.location_idx] = ByteCode::CallLocal { relative_off: (fun.offset - res.location_idx) as isize };
    }
    bc
}
