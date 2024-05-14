use std::{fmt::Debug, str::Chars};

use crate::{diagnostic_builder_spanned, span::Span};

pub fn lex(src: &str) -> anyhow::Result<Vec<TokenVal>> {
    let mut tokens = vec![];
    let mut buffer = String::new();
    let mut iter = LexingIter {
        inner: src.chars(),
        idx: 0,
    };
    let mut next_chr = iter.next();
    while let Some(chr) = next_chr {
        let start_idx = iter.idx;
        if chr == ' ' || chr == '\r' || chr == '\n' {
            next_chr = iter.next();
            continue;
        }
        if chr.is_numeric() {
            buffer.push(chr);
            let mut dots = 0;
            next_chr = collect_string_until(
                &mut iter,
                |chr| {
                    if chr == '.' {
                        dots += 1;
                    }
                    !chr.is_numeric()
                },
                &mut buffer,
            );
            if dots > 1 {
                return diagnostic_builder_spanned!(
                    "Found more than 1 dot while parsing number",
                    Span::single_token(iter.idx)
                );
            }
            tokens.push(TokenVal {
                token: Token::Number(core::mem::take(&mut buffer).parse::<f64>().unwrap()),
                span: Span::multi_token(start_idx, iter.idx),
            });
            continue;
        }
        if chr.is_alphabetic() {
            buffer.push(chr);
            next_chr = collect_string_until(
                &mut iter,
                |chr| !(chr.is_alphanumeric() || chr == '_'),
                &mut buffer,
            );
            let lit = core::mem::take(&mut buffer);
            let token = match lit.as_str() {
                "true" => Token::Bool(true),
                "false" => Token::Bool(false),
                "fn" => Token::Fn,
                "return" => Token::Return,
                "let" => Token::Let,
                "while" => Token::While,
                "if" => Token::If,
                "else" => Token::Else,
                _ => Token::Lit(lit),
            };
            tokens.push(TokenVal {
                token,
                span: Span::multi_token(start_idx, iter.idx),
            });
            continue;
        }
        if chr == '"' {
            collect_string_until(&mut iter, |chr| chr == '"', &mut buffer);
            next_chr = iter.next();
            tokens.push(TokenVal {
                token: Token::CharSeq(core::mem::take(&mut buffer)),
                span: Span::multi_token(start_idx, iter.idx),
            });
            continue;
        }
        let mut has_next = false;
        let token = match chr {
            '{' => Token::OpenCurly,
            '}' => Token::CloseCurly,
            '(' => Token::OpenBrace,
            ')' => Token::CloseBrace,
            ',' => Token::Comma,
            '!' => {
                next_chr = iter.next();
                match next_chr {
                    Some('=') => Token::Ne,
                    _ => {
                        has_next = true;
                        Token::Exclam
                    }
                }
            }
            '+' => Token::Add,
            '-' => Token::Sub,
            '*' => Token::Mul,
            '%' => Token::Mod,
            '/' => {
                next_chr = iter.next();
                match next_chr {
                    Some('/') => {
                        // skip comments
                        loop {
                            next_chr = iter.next();
                            if next_chr == Some('\n') || next_chr.is_none() {
                                break;
                            }
                        }
                        continue;
                    }
                    _ => {
                        has_next = true;
                        Token::Div
                    }
                }
            }
            '=' => {
                next_chr = iter.next();
                match next_chr {
                    Some('=') => Token::Eq,
                    _ => {
                        has_next = true;
                        Token::Assign
                    }
                }
            }
            '<' => {
                next_chr = iter.next();
                match next_chr {
                    Some('=') => Token::Le,
                    _ => {
                        has_next = true;
                        Token::Lt
                    }
                }
            }
            '>' => {
                next_chr = iter.next();
                match next_chr {
                    Some('=') => Token::Ge,
                    _ => {
                        has_next = true;
                        Token::Gt
                    }
                }
            }
            '&' => {
                next_chr = iter.next();
                match next_chr {
                    Some('&') => Token::And,
                    Some(chr) => {
                        return diagnostic_builder_spanned!(
                            format!("Expected `&` but found `{chr}`"),
                            Span::single_token(iter.idx)
                        )
                    }
                    None => {
                        return diagnostic_builder_spanned!(
                            "Expected `&` but found nothing",
                            Span::single_token(iter.idx)
                        )
                    }
                }
            }
            '|' => {
                next_chr = iter.next();
                match next_chr {
                    Some('|') => Token::Or,
                    Some(chr) => {
                        return diagnostic_builder_spanned!(
                            format!("Expected `|` but found `{chr}`"),
                            Span::single_token(iter.idx)
                        )
                    }
                    None => {
                        return diagnostic_builder_spanned!(
                            "Expected `|` but found nothing",
                            Span::single_token(iter.idx)
                        )
                    }
                }
            }
            _ => {
                return diagnostic_builder_spanned!(
                    format!("Unexpected character `{chr}`"),
                    Span::single_token(iter.idx)
                )
            }
        };
        tokens.push(TokenVal {
            token,
            span: Span::multi_token(start_idx, iter.idx - (if has_next { 1 } else { 0 })),
        });
        if !has_next {
            next_chr = iter.next();
        }
    }
    Ok(tokens)
}

struct LexingIter<'a> {
    inner: Chars<'a>,
    idx: usize,
}

impl<'a> Iterator for LexingIter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.inner.next();
        if ret.is_some() {
            self.idx += 1;
        }
        ret
    }
}

fn collect_string_until<F: FnMut(char) -> bool, I: Iterator<Item = char>>(
    src: &mut I,
    mut until: F,
    buffer: &mut String,
) -> Option<char> {
    while let Some(chr) = src.next() {
        if until(chr) {
            return Some(chr);
        }
        buffer.push(chr);
    }
    None
}

#[derive(Clone, Debug)]
pub struct TokenVal {
    pub token: Token,
    pub span: Span,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Token {
    Comma,  // ,
    Assign, // =
    Eq,     // Equals
    Ne,     // NotEquals
    Gt,     // GreaterThan
    Lt,     // LessThan
    Ge,     // GreaterEqual
    Le,     // LessEqual
    Exclam, // !
    And,    // &&
    Or,     // ||
    Div,    // /
    Mul,    // *
    Mod,    // %
    Add,    // +
    Sub,    // -
    OpenBrace,
    CloseBrace,
    OpenCurly,
    CloseCurly,
    Fn,
    Return,
    Let,
    While,
    If,
    Else,
    Lit(String),
    CharSeq(String),
    Number(f64),
    Bool(bool),
}

impl Token {
    pub fn kind(&self) -> TokenKind {
        match self {
            Token::Comma => TokenKind::Comma,
            Token::Assign => TokenKind::Assign,
            Token::OpenBrace => TokenKind::OpenBrace,
            Token::CloseBrace => TokenKind::CloseBrace,
            Token::OpenCurly => TokenKind::OpenCurly,
            Token::CloseCurly => TokenKind::CloseCurly,
            Token::CharSeq(_) => TokenKind::CharSeq,
            Token::Number(_) => TokenKind::Number,
            Token::Bool(_) => TokenKind::Bool,
            Token::Eq => TokenKind::Eq,
            Token::Ne => TokenKind::Ne,
            Token::Gt => TokenKind::Gt,
            Token::Lt => TokenKind::Lt,
            Token::Ge => TokenKind::Ge,
            Token::Le => TokenKind::Le,
            Token::Exclam => TokenKind::Exclam,
            Token::And => TokenKind::And,
            Token::Or => TokenKind::Or,
            Token::Let => TokenKind::Let,
            Token::While => TokenKind::While,
            Token::If => TokenKind::If,
            Token::Else => TokenKind::Else,
            Token::Lit(_) => TokenKind::Lit,
            Token::Div => TokenKind::Div,
            Token::Mul => TokenKind::Mul,
            Token::Mod => TokenKind::Mod,
            Token::Add => TokenKind::Add,
            Token::Sub => TokenKind::Sub,
            Token::Fn => TokenKind::Fn,
            Token::Return => TokenKind::Return,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    Comma,      // `,`
    Assign,     // `=`
    Eq,         // Equals `==`
    Ne,         // NotEquals `!=`
    Gt,         // GreaterThan `>`
    Lt,         // LessThan `<`
    Ge,         // GreaterEqual `>=`
    Le,         // LessEqual `<=`
    Exclam,     // `!`
    And,        // `&&`
    Or,         // `||`
    Div,        // /
    Mul,        // *
    Mod,        // %
    Add,        // +
    Sub,        // -
    OpenBrace,  // `(`
    CloseBrace, // `)`
    OpenCurly,  // `{`
    CloseCurly, // `}`
    Fn,
    Return,
    Let,
    While,
    If,
    Else,
    Lit,
    CharSeq,
    Number,
    Bool,
}
