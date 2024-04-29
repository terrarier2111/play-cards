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

pub enum PlayerSrc {
    Random,
    LastRound,
    NextRound,
    Select,
    // All,
}

#[derive(Clone, PartialEq)]
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

#[derive(Clone, Copy, PartialEq)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod, // modulo
    And,
    Or,
    Eq, // Equal
    Ne, // NotEqual
    Gt, // GreaterThan
    Lt, // LessThan
    Ge, // GreaterEqual
    Le, // LessEqual
}

#[derive(Clone, Copy, PartialEq)]
pub enum UnaryOpKind {
    Not,
}
