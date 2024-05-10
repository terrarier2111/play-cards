use std::{
    collections::HashMap,
    fs,
    num::NonZeroUsize,
    sync::{atomic::AtomicUsize, Arc},
};

use clitty::{
    core::{CLICore, CommandBuilder, CommandImpl, CommandParam, UsageBuilder},
    ui::{CLIBuilder, CmdLineInterface, PrintFallback},
};
use game_ctx::{CardTemplate, GameCtx, GameTemplate, PlayerDef};
use swap_it::{SwapArcOption, SwapGuard};

mod conc_once_cell;
mod funcs;
mod game_ctx;
mod sized_box;

static CTX: SwapArcOption<GameCtx> = SwapArcOption::new_empty();

fn main() {
    const PATH: &str = "./code/first.cgs";

    // FIXME: add UI
    let window = CLIBuilder::new()
        .command(
            CommandBuilder::new("play", CmdPlay).params(
                UsageBuilder::new()
                    .required(CommandParam {
                        name: "game",
                        ty: clitty::core::CommandParamTy::String(
                            clitty::core::CmdParamStrConstraints::None,
                        ),
                    })
                    .required(CommandParam {
                        name: "players",
                        ty: clitty::core::CommandParamTy::Unbound {
                            minimum: NonZeroUsize::new(2).unwrap(),
                            param: Box::new(clitty::core::CommandParamTy::String(
                                clitty::core::CmdParamStrConstraints::None,
                            )),
                        },
                    }),
            ),
        )
        .fallback(Box::new(PrintFallback::new(
            "This command is not known".to_string(),
        )))
        .prompt("Enter a command: ".to_string())
        .build();
    let cli = CmdLineInterface::new(window);
    cli.await_input(&()).unwrap();
}

pub fn get_ctx<'a>() -> SwapGuard<Arc<GameCtx>, GameCtx> {
    CTX.load().unwrap()
}

struct CmdPlay;

impl CommandImpl for CmdPlay {
    type CTX = ();

    fn execute(&self, _ctx: &Self::CTX, input: &[&str]) -> anyhow::Result<()> {
        let mut game: GameTemplate =
            serde_json::from_str(fs::read_to_string(input[0]).unwrap().as_str()).unwrap();
        let cards: Vec<CardTemplate> = game
            .card_paths
            .iter()
            .map(|path| serde_json::from_str(fs::read_to_string(path).unwrap().as_str()).unwrap())
            .collect::<Vec<_>>();
        game.cards = cards;
        let code_path = game.code_path.clone();
        let game = GameCtx {
            game,
            players: input
                .iter()
                .skip(1)
                .map(|player| PlayerDef {
                    name: player.to_string(),
                    inventories: vec![],
                    meta: HashMap::new(),
                    active: true,
                })
                .collect::<Vec<_>>(),
            inventories: vec![],
            draw_stack: vec![],
            meta: HashMap::new(),
            curr_player: AtomicUsize::new(0),
        };
        CTX.store(Arc::new(game));
        // start game
        engine::run(&code_path, vec![])?;
        Ok(())
    }
}
