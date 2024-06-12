use std::mem::transmute;

use crate::{
    parser::Stmt,
    rt::{Player, RtRef},
};

// end_game(player: Option<Player>)
// select_cards(cnt: usize, force_different: bool, allow_partial: bool)
// select_players(cnt: usize, src: PlayerSrc, allow_partial: bool)
// give_cards(inv: CardInventory, cards: Vec<CardVal>)
// setup_inv(vis: Visibility, slots: Option<usize>)

// FIXME: use implicit type conversions

/*pub enum Action {
    QueryUser {
        query: UserQuery,
    },
    GiveCards {
        inv: CardInventory,
        cards: Vec<CardVal>,
    },
    SetupInventory {
        vis: Visibility,
        slots: Option<usize>,
    },
    EndGame {
        winner: Option<Player>,
    },
}

pub enum UserQuery {
    SelectCards {
        cnt: usize,
        force_different: bool,
        allow_partial: bool,
    },
    SelectPlayers {
        cnt: usize,
        src: PlayerSrc,
        allow_partial: bool,
    },
}*/

#[derive(Clone, PartialEq, Debug)]
pub enum AstNode {
    CallFunc {
        name: String,
        params: Vec<AstNode>,
    },
    UnaryOp {
        val: Box<AstNode>,
        op: UnaryOpKind,
    },
    BinOp {
        lhs: Box<AstNode>,
        rhs: Box<AstNode>,
        op: BinOpKind,
    },
    Val(RtRef),
    Var {
        name: String,
    },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod, // Modulo
    And,
    Or,
    Eq, // Equal
    Ne, // NotEqual
    Gt, // GreaterThan
    Lt, // LessThan
    Ge, // GreaterEqual
    Le, // LessEqual
}

impl BinOpKind {
    pub fn priority(&self) -> usize {
        match self {
            BinOpKind::Add => 1,
            BinOpKind::Sub => 1,
            BinOpKind::Mul => 2,
            BinOpKind::Div => 2,
            BinOpKind::Mod => 2,
            BinOpKind::And => 0,
            BinOpKind::Or => 0,
            BinOpKind::Eq => 0,
            BinOpKind::Ne => 0,
            BinOpKind::Gt => 0,
            BinOpKind::Lt => 0,
            BinOpKind::Ge => 0,
            BinOpKind::Le => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UnaryOpKind {
    Not,
}
