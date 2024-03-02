pub enum Card {

}

pub enum Action {
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
    GiveCards {
        inv: String,
        cards: String,
    },
    SetupInventory {
        vis: Visibility,
        slots: Option<usize>,
    },
}

pub enum Visibility {
    None,
    Select(Vec<usize>),
    All,
}

pub enum CardInventory {
    Player(usize),
    Other(usize),
}

pub enum PlayerSrc {
    Random,
    LastRound,
    NextRound,
    Select,
    // All,
}