use std::{
    error::Error,
    fmt::{Debug, Display, Write},
    str::Chars,
};

use log::error;

pub fn lex(src: &str) -> anyhow::Result<Vec<Token>> {
    let mut tokens = vec![];
    let mut buffer = String::new();
    let mut iter = src.chars();
    let mut next_chr = iter.next();
    while let Some(chr) = next_chr {
        if chr == ' ' || chr == '\r' || chr == '\n' {
            next_chr = iter.next();
            continue;
        }
        if chr.is_numeric() {
            buffer.push(chr);
            let mut dot = false;
            collect_string_until(
                &mut iter,
                |chr| {
                    if chr == '.' {
                        if dot {
                            error!("Found more than 1 dot while parsing number");
                            panic!();
                        }
                        dot = true;
                    }
                    !chr.is_numeric()
                },
                &mut buffer,
            );
            next_chr = iter.next();
            tokens.push(Token::Number(
                core::mem::take(&mut buffer).parse::<f64>().unwrap(),
            ));
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
                "while" => Token::While,
                "if" => Token::If,
                "else" => Token::Else,
                _ => Token::Lit(lit),
            };
            tokens.push(token);
            continue;
        }
        if chr == '"' {
            collect_string_until(&mut iter, |chr| chr == '"', &mut buffer);
            next_chr = iter.next();
            tokens.push(Token::CharSeq(core::mem::take(&mut buffer)));
            continue;
        }
        let mut has_next = false;
        let token = match chr {
            '{' => Token::OpenCurly,
            '}' => Token::CloseCurly,
            '(' => Token::OpenBrace,
            ')' => Token::CloseBrace,
            ',' => Token::Comma,
            '!' => Token::Exclam,
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
                    _ => {
                        return Err(anyhow::Error::from(UnknownToken(chr))); // FIXME: improve diagnostics
                    }
                }
            }
            '|' => {
                next_chr = iter.next();
                match next_chr {
                    Some('|') => Token::Or,
                    _ => {
                        return Err(anyhow::Error::from(UnknownToken(chr))); // FIXME: improve diagnostics
                    }
                }
            }
            _ => return Err(anyhow::Error::from(UnknownToken(chr))),
        };
        tokens.push(token);
        if !has_next {
            next_chr = iter.next();
        }
    }
    Ok(tokens)
}

fn collect_string_until<F: FnMut(char) -> bool>(
    src: &mut Chars<'_>,
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
            Token::While => TokenKind::While,
            Token::If => TokenKind::If,
            Token::Else => TokenKind::Else,
            Token::Lit(_) => TokenKind::Lit,
            Token::Div => TokenKind::Div,
            Token::Mul => TokenKind::Mul,
            Token::Mod => TokenKind::Mod,
            Token::Add => TokenKind::Add,
            Token::Sub => TokenKind::Sub,
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
    While,
    If,
    Else,
    Lit,
    CharSeq,
    Number,
    Bool,
}

pub struct UnknownToken(char);

impl Error for UnknownToken {}

impl Display for UnknownToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl Debug for UnknownToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("found unknown character ")?;
        f.write_char(self.0)?;
        f.write_str(" while lexing program")
    }
}
