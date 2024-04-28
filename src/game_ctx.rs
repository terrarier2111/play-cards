use std::{collections::HashMap, sync::Arc};

use image::DynamicImage;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct GameTemplate {
    pub name: String,
    pub max_players: usize,
    pub min_players: usize,
    pub cards: Vec<CardTemplate>,
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
