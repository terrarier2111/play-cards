use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

use engine::{CardInventory, RtRef};
use image::DynamicImage;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct GameTemplate {
    pub name: String,
    pub max_players: usize,
    pub min_players: usize,
    #[serde(skip)]
    pub cards: Vec<CardTemplate>,
    pub card_paths: Vec<String>,
    pub code_path: String,
}

#[derive(Deserialize, Serialize)]
pub struct CardTemplate {
    pub name: String,
    pub ord: usize,
    pub image_path: String,
    #[serde(skip)]
    pub image: Arc<DynamicImage>,
    pub metadata: HashMap<String, String>,
}

pub struct GameCtx {
    pub game: GameTemplate,
    pub players: Vec<PlayerDef>,
    pub inventories: Vec<CardInventory>,
    pub draw_stack: Vec<usize>, // list of card indices
    pub meta: HashMap<String, RtRef>,
    pub curr_player: AtomicUsize,
}

pub struct PlayerDef {
    pub name: String,
    pub inventories: Vec<CardInventory>,
    pub meta: HashMap<String, RtRef>,
    pub active: bool,
}
