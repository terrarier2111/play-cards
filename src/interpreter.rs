use std::collections::HashMap;

use crate::{
    ast::AstNode,
    game_ctx::CardTemplate,
    parser::Stmt,
    rt::{CardVal, Ordering, RtRef, RtType},
};
use log::{error, warn};

pub struct Interpreter {
    // pub shared_vars: ,
    shared_funcs: HashMap<String, Box<dyn Fn(Vec<RtRef>) -> RtRef>>,
    vars: HashMap<String, RtRef>,
    cards: Vec<CardTemplate>,
    code: Vec<Stmt>,
}

impl Interpreter {
    pub fn new(cards: Vec<CardTemplate>, code: Vec<Stmt>) -> Self {
        Self {
            cards,
            code,
            vars: HashMap::new(),
            shared_funcs: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let mut idx = 0;
        loop {
            match self.code.get(idx).cloned() {
                Some(code) => {
                    self.eval_stmt(&code);
                    idx += 1;
                }
                None => break,
            }
        }
    }

    fn eval_node(&self, val: &AstNode) -> RtRef {
        match val {
            AstNode::CallFunc { name, params } => {
                let args = params
                    .iter()
                    .map(|arg| self.eval_node(arg))
                    .collect::<Vec<_>>();
                let val = match self.shared_funcs.get(name) {
                    Some(func) => func(args),
                    None => {
                        warn!("Couldn't find function {name}");
                        RtRef::NULL
                    }
                };
                val
            }
            AstNode::BinOp { lhs, rhs, op } => match op {
                crate::ast::BinOpKind::Add => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    RtRef::decimal(left.get_decimal().unwrap() + right.get_decimal().unwrap())
                }
                crate::ast::BinOpKind::Sub => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    RtRef::decimal(left.get_decimal().unwrap() - right.get_decimal().unwrap())
                }
                crate::ast::BinOpKind::Mul => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    RtRef::decimal(left.get_decimal().unwrap() * right.get_decimal().unwrap())
                }
                crate::ast::BinOpKind::Div => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    RtRef::decimal(left.get_decimal().unwrap() / right.get_decimal().unwrap())
                }
                crate::ast::BinOpKind::Mod => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    RtRef::decimal(
                        (left.get_decimal().unwrap() as usize
                            % right.get_decimal().unwrap() as usize) as f64,
                    )
                }
                crate::ast::BinOpKind::Eq => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    let ret = left.cmp_eq_vals(right);
                    RtRef::bool(ret.unwrap())
                }
                crate::ast::BinOpKind::Ne => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    let ret = left.cmp_eq_vals(right);
                    RtRef::bool(!ret.unwrap())
                }
                crate::ast::BinOpKind::Gt => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    let ret = left.cmp_vals(right);
                    RtRef::bool(ret.unwrap() == Ordering::Greater)
                }
                crate::ast::BinOpKind::Lt => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    let ret = left.cmp_vals(right);
                    RtRef::bool(ret.unwrap() == Ordering::Less)
                }
                crate::ast::BinOpKind::Ge => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    let ret = left.cmp_vals(right);
                    RtRef::bool(
                        ret.unwrap() == Ordering::Greater || ret.unwrap() == Ordering::Equal,
                    )
                }
                crate::ast::BinOpKind::Le => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    let ret = left.cmp_vals(right);
                    RtRef::bool(ret.unwrap() == Ordering::Less || ret.unwrap() == Ordering::Equal)
                }
                crate::ast::BinOpKind::And => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    RtRef::bool(left.get_bool().unwrap() && right.get_bool().unwrap())
                }
                crate::ast::BinOpKind::Or => {
                    let left = self.eval_node(lhs);
                    let right = self.eval_node(rhs);
                    RtRef::bool(left.get_bool().unwrap() || right.get_bool().unwrap())
                }
            },
            AstNode::Val(val) => *val,
        }
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> anyhow::Result<()> {
        match stmt {
            Stmt::DefineVar { name, val } => {
                let res = self.eval_node(&val);
                self.vars.insert(name.clone(), res);
            }
            Stmt::CallFunc { name, args } => {
                let args = args
                    .iter()
                    .map(|arg| self.eval_node(arg))
                    .collect::<Vec<_>>();
                let _ = match self.shared_funcs.get(name) {
                    Some(func) => func(args),
                    None => {
                        warn!("Couldn't find function {name}");
                        RtRef::NULL
                    }
                };
            }
            Stmt::Loop { stmts, condition } => loop {
                let result = self.eval_node(&condition);
                let result = match result.get_bool() {
                    Some(result) => result,
                    None => {
                        error!("Can't evaluate loop condition to boolean");
                        panic!();
                    }
                };
                if !result {
                    break;
                }
                for stmt in stmts.iter() {
                    self.eval_stmt(stmt)?;
                }
            },
            Stmt::Conditional { seq, fallback } => {
                for (condition, stmts) in seq {
                    let result = self.eval_node(condition);
                    let result = match result.get_bool() {
                        Some(result) => result,
                        None => {
                            error!("Can't evaluate loop condition to boolean");
                            panic!();
                        }
                    };
                    if result {
                        for stmt in stmts.iter() {
                            self.eval_stmt(stmt)?;
                        }
                        return Ok(());
                    }
                }
                for stmt in fallback.iter() {
                    self.eval_stmt(stmt)?;
                }
            }
        }
        Ok(())
    }
}

impl RtRef {
    pub fn cmp_eq_vals(self, other: Self) -> Option<bool> {
        if self.ty() != other.ty() {
            return None;
        }
        const FLOAT_DELTA: f64 = f64::EPSILON * 10.0;
        match self.ty() {
            RtType::None => Some(true),
            RtType::Player | RtType::Inventory => {
                Some((self.dst() as usize as u64) == (other.dst() as usize as u64))
            }
            RtType::Cards => Some(unsafe {
                (&*self.dst().cast::<Vec<CardVal>>())
                    .iter()
                    .eq((&*other.dst().cast::<Vec<CardVal>>()).iter())
            }),
            RtType::Decimal => Some(
                (unsafe { self.get_decimal_directly() - other.get_decimal_directly() })
                    < FLOAT_DELTA,
            ),
            RtType::Bool => Some(unsafe { self.get_bool_directly() == other.get_bool_directly() }),
            RtType::String => Some(
                unsafe { &*self.dst().cast::<String>() }
                    == unsafe { &*other.dst().cast::<String>() },
            ),
        }
    }

    pub fn cmp_vals(self, other: Self) -> Option<Ordering> {
        let ty = self.ty();
        if ty != other.ty() {
            return None;
        }
        match ty {
            RtType::None => Some(Ordering::Equal),
            RtType::Player | RtType::Inventory | RtType::Cards => {
                if (self.dst() as usize as u64) == (other.dst() as usize as u64) {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::NotEqual)
                }
            }
            RtType::Decimal => {
                match unsafe {
                    self.get_decimal_directly()
                        .partial_cmp(&other.get_decimal_directly())
                } {
                    Some(val) => match val {
                        std::cmp::Ordering::Less => Some(Ordering::Less),
                        std::cmp::Ordering::Equal => Some(Ordering::Equal),
                        std::cmp::Ordering::Greater => Some(Ordering::Greater),
                    },
                    None => Some(Ordering::NotEqual),
                }
            }
            RtType::Bool => Some(
                if unsafe { self.get_bool_directly() == other.get_bool_directly() } {
                    Ordering::Equal
                } else {
                    Ordering::NotEqual
                },
            ),
            RtType::String => Some(
                if unsafe { &*self.dst().cast::<String>() }
                    == unsafe { &*other.dst().cast::<String>() }
                {
                    Ordering::Equal
                } else {
                    Ordering::NotEqual
                },
            ),
        }
    }
}
