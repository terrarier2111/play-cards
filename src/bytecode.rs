use std::{collections::HashMap, mem, ptr};

use thin_vec::{thin_vec, ThinVec};

use crate::{
    ast::AstNode,
    parser::Stmt,
    rt::{RtRef, RtType},
};

#[repr(u8)]
pub enum ByteCode {
    Push {
        val: RtRef,
    },
    Pop,
    Call {
        fn_idx: u8,
        push_val: bool,
        /// we have to pass the pops here and perform it inside the call as we have to push the call's result right after popping the params
        param_pops: u8,
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
}

#[cfg(target_pointer_width = "64")]
pub type UHalf = u32;
#[cfg(target_pointer_width = "32")]
pub type UHalf = u16;
#[cfg(target_pointer_width = "16")]
pub type UHalf = u8;

pub struct Function {
    pub params: &'static [RtType],
    pub name: String,
    pub call: Box<dyn FnMut(&mut Vec<RtRef>) -> Option<RtRef>>,
}

struct Scope {
    vars: Vec<String>,
    stack_size: usize,
}

const TRUE_IDX: usize = 0;

pub fn translate(stmts: &Vec<Stmt>, fns: &Vec<Function>) -> Vec<ByteCode> {
    let mut code = vec![];
    let mut vars = HashMap::new();
    let mut stack_idx = 0;
    // used in jump conditional code to reverse the condition (this is pretty hacky but works and is fast)
    code.push(ByteCode::Push { val: RtRef::bool(true) });
    translate_internal(stmts, fns, &mut stack_idx, &mut vars, &mut code);
    code.push(ByteCode::Pop);
    code
}

fn translate_internal(
    stmts: &Vec<Stmt>,
    fns: &Vec<Function>,
    stack_idx: &mut usize,
    vars: &mut HashMap<String, usize>,
    code: &mut Vec<ByteCode>,
) {
    let mut curr_scope = Scope {
        vars: vec![],
        stack_size: 0,
    };
    for stmt in stmts {
        match stmt {
            Stmt::DefineVar { name, val } => {
                curr_scope.vars.push(name.clone());
                curr_scope.stack_size += 1;
                vars.insert(name.clone(), *stack_idx);
                *stack_idx += 1;

                // FIXME: evaluate `val` and push it on the stack
            }
            Stmt::CallFunc { name, args } => {
                let fn_idx = resolve_fn_idx(fns, name);

                // FIXME: check arg types
                if fns[fn_idx].params.len() != args.len() {
                    panic!("Function arg count mismatch (\"{}\")", fns[fn_idx].name);
                }

                let mut pops = 0;
                let mut indices = thin_vec![];
                for arg in args {
                    indices.push(translate_node(
                        &arg,
                        code,
                        &mut pops,
                        vars,
                        fns,
                        stack_idx,
                        &mut curr_scope.stack_size,
                    ) as UHalf);
                }
                code.push(ByteCode::Call {
                    fn_idx: fn_idx as u8,
                    push_val: false,
                    arg_indices: indices,
                    param_pops: pops as u8,
                });
                curr_scope.stack_size -= pops;
                *stack_idx -= pops;
            }
            Stmt::Loop { stmts, condition } => {
                let loop_start_len = code.len();
                let mut pops = 0;
                // this is the argument for the condition which decides whether to continue with the loop
                let arg_idx = translate_node(&condition, code, &mut pops, vars, fns, stack_idx, &mut curr_scope.stack_size);

                let prev_len = code.len();
                translate(stmts, fns);
                let body_size = code.len() - prev_len;

                // this includes the normal body size and all the additional code we generated for loop maintenance
                // the + 1 if from the unconditional Jump we use to go back to the condition at the end of the loop
                let full_body_size = body_size + pops + 1;
                
                // take the inverse of the condition
                code.insert(prev_len, ByteCode::Sub { arg1_idx: TRUE_IDX as UHalf, arg2_idx: arg_idx as UHalf });
                // skip the body if the inverse condition turns out to be true
                code.insert(prev_len + 1, ByteCode::JumpCond { relative_off: body_size as isize, arg_idx: arg_idx as UHalf });
                // cleanup for when we enter the loop
                for i in 0..pops {
                    code.insert(prev_len + 2 + i, ByteCode::Pop);
                }
                // go back to the beginning of the loop and retest its condition
                code.push(ByteCode::Jump { relative_off: -((code.len() - loop_start_len) as isize) });
                // cleanup for when we exit the loop
                for i in 0..pops {
                    code.push(ByteCode::Pop);
                }
            }
            Stmt::Conditional { seq, fallback } => {
                let mut condition_indices = vec![];
            }
        }
    }
    for var in curr_scope.vars {
        // FIXME: this is buggy as if there are multiple variables with the same name, it will come to collisions (if they are in different scopes)
        vars.remove(&var);
    }
    for _ in 0..curr_scope.stack_size {
        code.push(ByteCode::Pop);
    }
}

/// returns the corresponding stack index
fn translate_node(
    node: &AstNode,
    code: &mut Vec<ByteCode>,
    pops: &mut usize,
    vars: &HashMap<String, usize>,
    funcs: &Vec<Function>,
    stack_idx: &mut usize,
    curr_stack_frame_size: &mut usize,
) -> usize {
    match node {
        AstNode::CallFunc { name, params } => {
            let func_idx = resolve_fn_idx(funcs, name);

            let mut call_pops = 0;
            let mut indices = thin_vec![];
            for param in params {
                indices.push(translate_node(
                    param,
                    code,
                    &mut call_pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                ) as UHalf);
            }

            code.push(ByteCode::Call {
                fn_idx: func_idx as u8,
                push_val: true,
                param_pops: call_pops as u8,
                arg_indices: indices,
            }); // FIXME: should we push val?
            *pops += 1;
            *stack_idx += 1;
            *curr_stack_frame_size += 1;

            *stack_idx -= call_pops;
            *curr_stack_frame_size -= call_pops;

            *stack_idx - 1
        }
        AstNode::BinOp { lhs, rhs, op } => match op {
            crate::ast::BinOpKind::Add => {
                let idx1 = translate_node(
                    lhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                let idx2 = translate_node(
                    rhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                code.push(ByteCode::Add {
                    arg1_idx: idx1 as UHalf,
                    arg2_idx: idx2 as UHalf,
                });
                *pops += 1;
                *stack_idx += 1;
                *curr_stack_frame_size += 1;
                *stack_idx - 1
            }
            crate::ast::BinOpKind::Sub => {
                let idx1 = translate_node(
                    lhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                let idx2 = translate_node(
                    rhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                code.push(ByteCode::Sub {
                    arg1_idx: idx1 as UHalf,
                    arg2_idx: idx2 as UHalf,
                });
                *pops += 1;
                *stack_idx += 1;
                *curr_stack_frame_size += 1;
                *stack_idx - 1
            }
            crate::ast::BinOpKind::Mul => {
                let idx1 = translate_node(
                    lhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                let idx2 = translate_node(
                    rhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                code.push(ByteCode::Mul {
                    arg1_idx: idx1 as UHalf,
                    arg2_idx: idx2 as UHalf,
                });
                *pops += 1;
                *stack_idx += 1;
                *curr_stack_frame_size += 1;
                *stack_idx - 1
            }
            crate::ast::BinOpKind::Div => {
                let idx1 = translate_node(
                    lhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                let idx2 = translate_node(
                    rhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                code.push(ByteCode::Div {
                    arg1_idx: idx1 as UHalf,
                    arg2_idx: idx2 as UHalf,
                });
                *pops += 1;
                *stack_idx += 1;
                *curr_stack_frame_size += 1;
                *stack_idx - 1
            }
            crate::ast::BinOpKind::Mod => {
                let idx1 = translate_node(
                    lhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                let idx2 = translate_node(
                    rhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                code.push(ByteCode::Mod {
                    arg1_idx: idx1 as UHalf,
                    arg2_idx: idx2 as UHalf,
                });
                *pops += 1;
                *stack_idx += 1;
                *curr_stack_frame_size += 1;
                *stack_idx - 1
            }
            crate::ast::BinOpKind::And => {
                let idx1 = translate_node(
                    lhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                let idx2 = translate_node(
                    rhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                code.push(ByteCode::And {
                    arg1_idx: idx1 as UHalf,
                    arg2_idx: idx2 as UHalf,
                });
                *pops += 1;
                *stack_idx += 1;
                *curr_stack_frame_size += 1;
                *stack_idx - 1
            }
            crate::ast::BinOpKind::Or => {
                let idx1 = translate_node(
                    lhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                let idx2 = translate_node(
                    rhs,
                    code,
                    pops,
                    vars,
                    funcs,
                    stack_idx,
                    curr_stack_frame_size,
                );
                code.push(ByteCode::Or {
                    arg1_idx: idx1 as UHalf,
                    arg2_idx: idx2 as UHalf,
                });
                *pops += 1;
                *stack_idx += 1;
                *curr_stack_frame_size += 1;
                *stack_idx - 1
            }
            crate::ast::BinOpKind::Eq => todo!(),
            crate::ast::BinOpKind::Ne => todo!(),
            crate::ast::BinOpKind::Gt => todo!(),
            crate::ast::BinOpKind::Lt => todo!(),
            crate::ast::BinOpKind::Ge => todo!(),
            crate::ast::BinOpKind::Le => todo!(),
        },
        AstNode::Val(val) => {
            code.push(ByteCode::Push { val: *val });
            *pops += 1;
            *stack_idx += 1;
            *curr_stack_frame_size += 1;
            *stack_idx - 1
        }
        AstNode::Var { name } => *vars.get(name).unwrap(),
    }
}

// FIXME: handle missing functions
fn resolve_fn_idx(fns: &Vec<Function>, fn_name: &String) -> usize {
    let fn_idx = fns
        .iter()
        .enumerate()
        .find(|(_, val)| &val.name == fn_name)
        .unwrap()
        .0;
    fn_idx
}
