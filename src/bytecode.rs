use std::{collections::HashMap, mem, ptr};

use crate::{ast::{AstNode, Function}, parser::Stmt, rt::RtRef};

pub enum ByteCode {
    Push { val: RtRef },
    Pop,
    Call { fn_idx: UHalf,
            push_val: bool,
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
    Jump { relative_off: isize },
    // the condition is stored on the stack
    JumpCond { relative_off: isize },
}

#[cfg(target_pointer_width = "64")]
pub type UHalf = u32;
#[cfg(target_pointer_width = "32")]
pub type UHalf = u16;
#[cfg(target_pointer_width = "16")]
pub type UHalf = u8;

pub struct RegPair {
    pub first: UHalf,
    pub second: UHalf,
}

pub fn translate(stmts: Vec<Stmt>, fns: &Vec<(String, Box<dyn FnMut(&mut Vec<RtRef>) -> Option<RtRef>>)>) -> Vec<ByteCode> {
    let mut code = vec![];
    let mut vars = HashMap::new();
    let mut scopes = vec![];
    let mut stack_idx = 0;
    let mut curr_stack_frame_size = 0;
    for stmt in stmts {
        match stmt {
            Stmt::DefineVar { name, val } => {
                scopes.last_mut().unwrap().push(name.clone());
                vars.insert(name, stack_idx);
                stack_idx += 1;
                curr_stack_frame_size += 1;
                // FIXME: evaluate `val` and push it on the stack
            },
            Stmt::CallFunc { name, args } => {
                let fn_idx = fns.iter().enumerate().find(|(idx, val)| {
                    &val.0 == &name
                }).unwrap().0; // FIXME: handle missing functions
                // FIXME: translate args and push them on the stack
                code.push(ByteCode::Call { fn_idx: fn_idx as UHalf, push_val: false });
            },
            Stmt::Loop { stmts, condition } => {
                scopes.push(vec![]);
                let idx = code.len() - 1;
                translate(stmts, fns);
                // FIXME: translate condition to a conditional jump to idx
            },
            Stmt::Conditional { seq, fallback } => {
                let mut condition_indices = vec![];

            },
        }
    }
    code
}

/// returns the corresponding stack index
fn translate_node(node: &AstNode, code: &mut Vec<ByteCode>, pops: &mut usize, vars: &HashMap<String, usize>, funcs: HashMap<String, Function>, stack_idx: &mut usize) -> usize {
    match node {
        AstNode::CallFunc { name, params } => {
            let func = funcs.get(name).unwrap();

            code.push(ByteCode::Call { fn_idx: func.idx, push_val: true }); // FIXME: should we push val?
            *pops += 1;
            *stack_idx += 1;

            *stack_idx - 1
        },
        AstNode::BinOp { lhs, rhs, op } => {
            match op {
                crate::ast::BinOpKind::Add => {
                    let idx1 = translate_node(lhs, code, pops, vars, funcs, stack_idx);
                    let idx2 = translate_node(rhs, code, pops, vars, funcs, stack_idx);
                    code.push(ByteCode::Add {
                        arg1_idx: idx1,
                        arg2_idx: idx2,
                    });
                },
                crate::ast::BinOpKind::Sub => {
                    let idx1 = translate_node(lhs, code, pops, vars, funcs, stack_idx);
                    let idx2 = translate_node(rhs, code, pops, vars, funcs, stack_idx);
                    code.push(ByteCode::Sub {
                        arg1_idx: idx1,
                        arg2_idx: idx2,
                    });
                },
                crate::ast::BinOpKind::Mul => {
                    let idx1 = translate_node(lhs, code, pops, vars, funcs, stack_idx);
                    let idx2 = translate_node(rhs, code, pops, vars, funcs, stack_idx);
                    code.push(ByteCode::Mul {
                        arg1_idx: idx1,
                        arg2_idx: idx2,
                    });
                },
                crate::ast::BinOpKind::Div => {
                    let idx1 = translate_node(lhs, code, pops, vars, funcs, stack_idx);
                    let idx2 = translate_node(rhs, code, pops, vars, funcs, stack_idx);
                    code.push(ByteCode::Div {
                        arg1_idx: idx1,
                        arg2_idx: idx2,
                    });
                },
                crate::ast::BinOpKind::Mod => {
                    let idx1 = translate_node(lhs, code, pops, vars, funcs, stack_idx);
                    let idx2 = translate_node(rhs, code, pops, vars, funcs, stack_idx);
                    code.push(ByteCode::Mod {
                        arg1_idx: idx1,
                        arg2_idx: idx2,
                    });
                },
                crate::ast::BinOpKind::And => {
                    let idx1 = translate_node(lhs, code, pops, vars, funcs, stack_idx);
                    let idx2 = translate_node(rhs, code, pops, vars, funcs, stack_idx);
                    code.push(ByteCode::And {
                        arg1_idx: idx1,
                        arg2_idx: idx2,
                    });
                },
                crate::ast::BinOpKind::Or => {
                    let idx1 = translate_node(lhs, code, pops, vars, funcs, stack_idx);
                    let idx2 = translate_node(rhs, code, pops, vars, funcs, stack_idx);
                    code.push(ByteCode::Or {
                        arg1_idx: idx1,
                        arg2_idx: idx2,
                    });
                },
                crate::ast::BinOpKind::Eq => todo!(),
                crate::ast::BinOpKind::Ne => todo!(),
                crate::ast::BinOpKind::Gt => todo!(),
                crate::ast::BinOpKind::Lt => todo!(),
                crate::ast::BinOpKind::Ge => todo!(),
                crate::ast::BinOpKind::Le => todo!(),
            }
        },
        AstNode::Val(val) => {
            code.push(ByteCode::Push { val: *val });
            *pops += 1;
            *stack_idx += 1;
            *stack_idx - 1
        },
        AstNode::Var { name } => vars.get(name).unwrap(),
    }
}
