use std::sync::atomic::Ordering;

use engine::{Player, RtRef};

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

pub fn create_inv(args: Vec<RtRef>) -> Option<RtRef> {
    todo!()
}
