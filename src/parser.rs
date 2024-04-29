use crate::{
    ast::{AstNode, UnaryOpKind},
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

    fn parse_loop(&mut self) -> anyhow::Result<Stmt> {
        let cond = self.try_parse_bin_op()?;
        if !self.try_eat(TokenKind::OpenCurly) {
            panic!("Error parsing loop");
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
                panic!("Error parsing if");
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
                    panic!("Error parsing else");
                }
                let mut stmts = vec![];
                while !self.try_eat(TokenKind::CloseCurly) {
                    stmts.push(self.parse_stmt()?);
                }
                fallback = Some(stmts);
            }
            break;
        }

        Ok(Stmt::Conditional { seq: conditions, fallback: fallback.unwrap_or(vec![]) })
    }

    fn parse_stmt(&mut self) -> anyhow::Result<Stmt> {
        match self.next().unwrap() {
            Token::OpenCurly => todo!(),
            Token::While => self.parse_loop(),
            Token::If => self.parse_if(),
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
                                panic!("Can't parse func");
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
                        })
                    }
                    _ => panic!("Can't parse var or func"),
                }
            }
            token => panic!("didn't expect token {:?}", token),
        }
    }

    fn try_parse_bin_op(&mut self) -> anyhow::Result<AstNode> {
        let lhs = match self.next() {
            Some(token) => match token {
                Token::Exclam => return Ok(AstNode::UnaryOp { val: Box::new(self.try_parse_bin_op()?), op: UnaryOpKind::Not }),
                Token::OpenBrace => {
                    let op = self.try_parse_bin_op()?;
                    if !self.try_eat(TokenKind::CloseBrace) {
                        panic!("Can't find closing brace");
                    }
                    op
                },
                Token::OpenCurly => todo!(),
                Token::Lit(val) => AstNode::Var { name: val },
                Token::CharSeq(_) => AstNode::Val(RtRef::),
                Token::Number(val) => AstNode::Val(RtRef::decimal(val)),
                Token::Bool(val) => AstNode::Val(RtRef::bool(val)),
                _ => unreachable!(),
            },
            None => unreachable!(),
        };
        if matches!(self.look_ahead().map(|token| token.kind()), Some(TokenKind::Add | TokenKind::Sub | TokenKind::And | TokenKind::Div | TokenKind::Mul | TokenKind::Mod | TokenKind::Or)) {
            match self.look_ahead().unwrap().kind() {
                TokenKind::Comma => todo!(),
                TokenKind::Semi => todo!(),
                TokenKind::Assign => todo!(),
                TokenKind::Eq => todo!(),
                TokenKind::Ne => todo!(),
                TokenKind::Gt => todo!(),
                TokenKind::Lt => todo!(),
                TokenKind::Ge => todo!(),
                TokenKind::Le => todo!(),
                TokenKind::Exclam => todo!(),
                TokenKind::And => todo!(),
                TokenKind::Or => todo!(),
                TokenKind::Div => todo!(),
                TokenKind::Mul => todo!(),
                TokenKind::Mod => todo!(),
                TokenKind::Add => todo!(),
                TokenKind::Sub => todo!(),
                TokenKind::OpenBrace => todo!(),
                TokenKind::CloseBrace => todo!(),
                TokenKind::OpenCurly => todo!(),
                TokenKind::CloseCurly => todo!(),
                TokenKind::While => todo!(),
                TokenKind::If => todo!(),
                TokenKind::Else => todo!(),
                TokenKind::Lit => todo!(),
                TokenKind::CharSeq => todo!(),
                TokenKind::Number => todo!(),
                TokenKind::Bool => todo!(),
            }
        }
        todo!()
    }
}

pub fn parse(tokens: Vec<Token>) -> anyhow::Result<Vec<Stmt>> {
    let mut parser = Parser { idx: 0, tokens };
    let mut stmts = vec![];
    while parser.idx < parser.tokens.len() {
        stmts.push(parser.parse_stmt()?);
    }
    Ok(stmts)
}

#[derive(Clone)]
pub enum Stmt {
    DefineVar {
        name: String,
        val: AstNode,
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
