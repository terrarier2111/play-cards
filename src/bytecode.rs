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

pub fn translate(stmts: &Vec<Stmt>, fns: &Vec<Function>) -> Vec<ByteCode> {
    let mut code = vec![];
    let mut vars = HashMap::new();
    let mut stack_idx = 0;
    translate_internal(stmts, fns, &mut stack_idx, &mut vars, &mut code);
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
            }
            Stmt::Loop { stmts, condition } => {
                let idx = code.len() - 1;
                translate(stmts, fns);
                // FIXME: translate condition to a conditional jump to idx
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
