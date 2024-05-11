use std::{error::Error, fmt::{Debug, Display}, mem};

use crate::{
    ast::{AstNode, BinOpKind, UnaryOpKind},
    lexer::{Token, TokenKind},
    rt::RtRef,
};

struct Parser {
    idx: usize,
    tokens: Vec<Token>,
}

impl Parser {
    fn next(&mut self) -> Option<Token> {
        self.idx += 1;
        self.tokens.get(self.idx - 1).cloned()
    }

    fn look_ahead(&self) -> Option<Token> {
        self.tokens.get(self.idx).cloned()
    }

    fn try_eat(&mut self, token_kind: TokenKind) -> bool {
        let ret = self
            .tokens
            .get(self.idx)
            .map(|val| val.kind() == token_kind)
            .unwrap_or(false);
        if ret {
            self.idx += 1;
        }
        ret
    }

    fn parse_lit(&mut self) -> Option<String> {
        if let Some(Token::Lit(lit)) = self.next() {
            Some(lit)
        } else {
            None
        }
    }

    fn parse_loop(&mut self) -> anyhow::Result<Stmt> {
        let cond = self.try_parse_bin_op()?;
        if !self.try_eat(TokenKind::OpenCurly) {
            return Err(error("Missing `{` in loop".to_string()));
        }
        let mut stmts = vec![];
        while !self.try_eat(TokenKind::CloseCurly) {
            stmts.push(self.parse_stmt()?);
        }
        Ok(Stmt::Loop {
            stmts,
            condition: Box::new(cond),
        })
    }

    fn parse_if(&mut self) -> anyhow::Result<Stmt> {
        let mut conditions = vec![];
        let mut fallback = None;
        loop {
            let cond = self.try_parse_bin_op()?;
            if !self.try_eat(TokenKind::OpenCurly) {
                return Err(error("Missing `{` in if".to_string()));
            }
            let mut stmts = vec![];
            while !self.try_eat(TokenKind::CloseCurly) {
                stmts.push(self.parse_stmt()?);
            }
            conditions.push((cond, stmts));
            if self.try_eat(TokenKind::Else) {
                if self.try_eat(TokenKind::If) {
                    continue;
                }
                if !self.try_eat(TokenKind::OpenCurly) {
                    return Err(error("Missing `{` in else".to_string()));
                }
                let mut stmts = vec![];
                while !self.try_eat(TokenKind::CloseCurly) {
                    stmts.push(self.parse_stmt()?);
                }
                fallback = Some(stmts);
            }
            break;
        }

        Ok(Stmt::Conditional {
            seq: conditions,
            fallback: fallback.unwrap_or(vec![]),
        })
    }

    fn parse_let(&mut self) -> anyhow::Result<Stmt> {
        let name = self.parse_lit().unwrap();
        if !self.try_eat(TokenKind::Assign) {
            return Err(error("Missing `=` in let".to_string()));
        }
        Ok(Stmt::DefineVar {
            name,
            val: self.try_parse_bin_op()?,
            reassign: false,
        })
    }

    fn parse_stmt(&mut self) -> anyhow::Result<Stmt> {
        match self.next().unwrap() {
            Token::OpenCurly => todo!(),
            Token::While => self.parse_loop(),
            Token::If => self.parse_if(),
            Token::Let => self.parse_let(),
            Token::Lit(var) => {
                match self.next() {
                    Some(Token::OpenBrace) => {
                        // parse function call
                        let mut params = vec![];
                        loop {
                            if self.try_eat(TokenKind::CloseBrace) {
                                break;
                            }
                            params.push(self.try_parse_bin_op()?);
                            if !self.try_eat(TokenKind::Comma) {
                                if self.try_eat(TokenKind::CloseBrace) {
                                    break;
                                }
                                return Err(error("Missing `)` to match `(` for function calls".to_string()));
                            }
                        }
                        Ok(Stmt::CallFunc {
                            name: var,
                            args: params,
                        })
                    }
                    Some(Token::Assign) => {
                        // parse variable definition
                        Ok(Stmt::DefineVar {
                            name: var,
                            val: self.try_parse_bin_op()?,
                            reassign: true,
                        })
                    }
                    token => return Err(error(format!("Can't parse variable or function, expected `(` or `=`, but found `{:?}`", token))),
                }
            }
            token => return Err(error(format!("Didn't expect `{:?}` when parsing statement", token))),
        }
    }

    fn try_parse_bin_op(&mut self) -> anyhow::Result<AstNode> {
        let mut lhs = match self.next() {
            Some(token) => match token {
                Token::Exclam => {
                    return Ok(AstNode::UnaryOp {
                        val: Box::new(self.try_parse_bin_op()?),
                        op: UnaryOpKind::Not,
                    })
                }
                Token::OpenBrace => {
                    let op = self.try_parse_bin_op()?;
                    if !self.try_eat(TokenKind::CloseBrace) {
                        return Err(error("Missing `)` to match `(`".to_string()));
                    }
                    op
                }
                Token::OpenCurly => todo!(),
                Token::Lit(val) => AstNode::Var { name: val },
                Token::CharSeq(val) => AstNode::Val(RtRef::string(Box::new(val))),
                Token::Number(val) => AstNode::Val(RtRef::decimal(val)),
                Token::Bool(val) => AstNode::Val(RtRef::bool(val)),
                token => return Err(error(format!("found unexpected token {:?} when parsing binop", token))),
            },
            None => unreachable!(),
        };
        if !matches!(
            self.look_ahead().map(|token| token.kind()),
            Some(
                TokenKind::Add
                    | TokenKind::Sub
                    | TokenKind::And
                    | TokenKind::Div
                    | TokenKind::Mul
                    | TokenKind::Mod
                    | TokenKind::Or
                    | TokenKind::Eq
                    | TokenKind::Ne
                    | TokenKind::Gt
                    | TokenKind::Ge
                    | TokenKind::Lt
                    | TokenKind::Le
            )
        ) {
            return Ok(lhs);
        }
        let bin_op = match self.look_ahead().unwrap().kind() {
            TokenKind::Eq => BinOpKind::Eq,
            TokenKind::Ne => BinOpKind::Ne,
            TokenKind::Gt => BinOpKind::Gt,
            TokenKind::Lt => BinOpKind::Lt,
            TokenKind::Ge => BinOpKind::Ge,
            TokenKind::Le => BinOpKind::Le,
            TokenKind::And => BinOpKind::And,
            TokenKind::Or => BinOpKind::Or,
            TokenKind::Div => BinOpKind::Div,
            TokenKind::Mul => BinOpKind::Mul,
            TokenKind::Mod => BinOpKind::Mod,
            TokenKind::Add => BinOpKind::Add,
            TokenKind::Sub => BinOpKind::Sub,
            token => unreachable!("found unexpected token {:?}", token),
        };
        // eat bin_op token
        self.next();
        let rhs = self.try_parse_bin_op()?;
        let mut nodes = vec![];
        let mut ops = vec![];
        if let AstNode::BinOp { lhs, rhs, op } = lhs {
            nodes.push(lhs);
            nodes.push(rhs);
            ops.push(op);
        } else {
            nodes.push(Box::new(lhs));
        }
        ops.push(bin_op);
        if let AstNode::BinOp { lhs, rhs, op } = rhs {
            nodes.push(lhs);
            nodes.push(rhs);
            ops.push(op);
        } else {
            nodes.push(Box::new(rhs));
        }

        let mut finished_nodes = vec![];
        while !ops.is_empty() {
            let mut highest_idx = 0;
            let mut highest_prio = 0;
            for op in ops.iter().enumerate() {
                if op.1.priority() > highest_prio {
                    highest_prio = op.1.priority();
                    highest_idx = op.0;
                }
            }
            let lhs = if !nodes.is_empty() { nodes.remove(highest_idx) } else { finished_nodes.remove(0) };
            let rhs = if !nodes.is_empty() { nodes.remove(highest_idx) } else { finished_nodes.remove(0) };
            let op = ops.remove(highest_idx);
            finished_nodes.push(Box::new(AstNode::BinOp { lhs: lhs, rhs: rhs, op }));
        }


        Ok(*finished_nodes.pop().unwrap())
    }
}

fn error(val: String) -> anyhow::Error {
    anyhow::Error::new(ParseError(val))
}

pub fn parse(tokens: Vec<Token>) -> anyhow::Result<Vec<Stmt>> {
    let mut parser = Parser { idx: 0, tokens };
    let mut stmts = vec![];
    while parser.idx < parser.tokens.len() {
        stmts.push(parser.parse_stmt()?);
    }
    Ok(stmts)
}

#[derive(Clone, Debug)]
pub enum Stmt {
    DefineVar {
        name: String,
        val: AstNode,
        reassign: bool,
    },
    CallFunc {
        name: String,
        args: Vec<AstNode>,
    },
    Loop {
        stmts: Vec<Stmt>,
        condition: Box<AstNode>,
    },
    Conditional {
        seq: Vec<(AstNode, Vec<Stmt>)>,
        fallback: Vec<Stmt>,
    },
}

pub struct ParseError(String);

impl Error for ParseError {}

impl Debug for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}