use crate::{
    ast::AstNode,
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
        let cond = self.parse_node()?;
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
            let cond = self.parse_node()?;
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
                            params.push(self.parse_node()?);
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
                            val: self.parse_node()?,
                        })
                    }
                    _ => panic!("Can't parse var or func"),
                }
            }
            token => panic!("didn't expect token {:?}", token),
        }
    }

    fn parse_node(&mut self) -> anyhow::Result<AstNode> {
        match self.next() {
            Some(token) => match token {
                Token::Exclam => todo!(),
                Token::OpenBrace => todo!(),
                Token::OpenCurly => todo!(),
                Token::Lit(_) => todo!(),
                Token::CharSeq(_) => todo!(),
                Token::Number(_) => todo!(),
                Token::Bool(_) => todo!(),
                _ => unreachable!(),
            },
            None => unreachable!(),
        }
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
