use std::sync::atomic::Ordering;

use engine::{CardInventory, CardInventoryRef, Player, RtRef};

use crate::get_ctx;

pub fn next_player(_args: Vec<RtRef>) -> Option<RtRef> {
    let ctx = get_ctx();
    let mut curr_player = ctx.curr_player.load(Ordering::Acquire);
    loop {
        curr_player = (curr_player + 1) % ctx.players.len();
        if ctx.players[curr_player].active {
            ctx.curr_player.store(curr_player, Ordering::Release);
            return Some(RtRef::player(Player::new(curr_player as u64)));
        }
    }
}

pub fn player_cnt(_args: Vec<RtRef>) -> Option<RtRef> {
    let mut players = 0;
    for player in get_ctx().players.iter() {
        if player.active {
            players += 1;
        }
    }
    Some(RtRef::decimal(players as f64))
}

pub fn player_name(args: Vec<RtRef>) -> Option<RtRef> {
    let player = if args.is_empty() {
        Player::new(get_ctx().curr_player.load(Ordering::Acquire) as u64)
    } else {
        args.first().unwrap().get_player().unwrap()
    };
    Some(RtRef::string(Box::new(
        get_ctx().players[player.idx() as usize].name.clone(),
    )))
}

pub fn create_inv_global(args: Vec<RtRef>) -> Option<RtRef> {
    let slots = args[0].get_decimal().unwrap();
    get_ctx().inventories.lock().unwrap().push(CardInventory {
        slots: slots as i64 as u64,
        vis: None,
        cards: vec![],
    });
    Some(RtRef::inventory(CardInventoryRef(
        (get_ctx().inventories.lock().unwrap().len() - 1) as u64,
    )))
}

pub fn create_inv_restricted(args: Vec<RtRef>) -> Option<RtRef> {
    let slots = args[0].get_decimal().unwrap();
    let players = args
        .iter()
        .skip(1)
        .map(|val| val.get_player().unwrap())
        .collect::<Vec<_>>();
    get_ctx().inventories.lock().unwrap().push(CardInventory {
        slots: slots as i64 as u64,
        vis: Some(players),
        cards: vec![],
    });
    Some(RtRef::inventory(CardInventoryRef(
        (get_ctx().inventories.lock().unwrap().len() - 1) as u64,
    )))
}

pub fn store_meta(args: Vec<RtRef>) -> Option<RtRef> {
    assert!(args.len() < 4 && args.len() > 1);
    if args.len() == 3 {
        let player = args[0].get_player().unwrap();
        let meta_name = args[1].get_string().unwrap();
        let meta_val = args[2];
        get_ctx().players[player.idx() as usize]
            .meta
            .lock()
            .unwrap()
            .insert(meta_name.clone(), meta_val);
    } else {
        // FIXME: insert into game
    }
    None
}

pub fn load_meta(args: Vec<RtRef>) -> Option<RtRef> {
    assert!(args.len() < 4 && args.len() > 1);
    if args.len() == 2 {
        let player = args[0].get_player().unwrap();
        let meta_name = args[1].get_string().unwrap();
        get_ctx().players[player.idx() as usize]
            .meta
            .lock()
            .unwrap()
            .get(meta_name)
            .cloned()
    } else {
        // FIXME: load from game
        todo!()
    }
}

pub fn player_play(args: Vec<RtRef>) -> Option<RtRef> {
    todo!()
}
