use std::{
    collections::HashMap,
    fs,
    num::{NonZero, NonZeroUsize},
    path::Path,
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

use clitty::{
    core::{
        CLICore, CmdParamNumConstraints, CmdParamStrConstraints, CommandBuilder, CommandImpl,
        CommandParam, CommandParamTy, UsageBuilder,
    },
    ui::{CLIBuilder, CmdLineInterface, PrintFallback},
};
use conc_once_cell::ConcurrentOnceCell;
use engine::{Function, RtType};
use funcs::{create_inv_global, create_inv_restricted, load_meta, next_player, player_cnt, player_name, store_meta};
use game_ctx::{CardTemplate, GameCtx, GameTemplate, PlayerDef};
use image::DynamicImage;
use swap_it::{SwapArcOption, SwapGuard};

mod conc_once_cell;
mod funcs;
mod game_ctx;
mod sized_box;

static CTX: SwapArcOption<GameCtx> = SwapArcOption::new_empty();
static CLI: ConcurrentOnceCell<CmdLineInterface<()>> = ConcurrentOnceCell::new();

fn main() {
    fs::create_dir_all(GAMES_DIR).unwrap();
    fs::create_dir_all(CARDS_DIR).unwrap();

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
                            minimum: NonZeroUsize::new(1).unwrap(),
                            param: Box::new(clitty::core::CommandParamTy::String(
                                clitty::core::CmdParamStrConstraints::None,
                            )),
                        },
                    }),
            ),
        )
        .command(
            CommandBuilder::new("create", CmdCreate).params(
                UsageBuilder::new()
                    .required(CommandParam {
                        name: "name",
                        ty: CommandParamTy::String(CmdParamStrConstraints::None),
                    })
                    .required(CommandParam {
                        name: "code path",
                        ty: CommandParamTy::String(CmdParamStrConstraints::None),
                    })
                    .required(CommandParam {
                        name: "min players",
                        ty: CommandParamTy::UInt(CmdParamNumConstraints::None),
                    })
                    .required(CommandParam {
                        name: "max players",
                        ty: CommandParamTy::UInt(CmdParamNumConstraints::None),
                    })
                    .required(CommandParam {
                        name: "card names",
                        ty: CommandParamTy::Unbound {
                            minimum: NonZero::new(1).unwrap(),
                            param: Box::new(CommandParamTy::String(CmdParamStrConstraints::None)),
                        },
                    }),
            ),
        )
        .command(CommandBuilder::new("games", CmdGames))
        .command(
            CommandBuilder::new("mkcard", CmdCreateCard).params(
                UsageBuilder::new()
                    .required(CommandParam {
                        name: "name",
                        ty: CommandParamTy::String(CmdParamStrConstraints::None),
                    })
                    .required(CommandParam {
                        name: "ordinal",
                        ty: CommandParamTy::UInt(CmdParamNumConstraints::None),
                    })
                    .required(CommandParam {
                        name: "image path",
                        ty: CommandParamTy::String(CmdParamStrConstraints::None),
                    }),
            ),
        )
        .fallback(Box::new(PrintFallback::new(
            "This command is not known".to_string(),
        )))
        .prompt("Enter a command: ".to_string())
        .build();
    CLI.get_or_init(|| CmdLineInterface::new(window));
    loop {
        CLI.get().unwrap().await_input(&()).unwrap();
    }
}

pub fn get_ctx<'a>() -> SwapGuard<Arc<GameCtx>, GameCtx> {
    CTX.load().unwrap()
}

struct CmdPlay;

impl CommandImpl for CmdPlay {
    type CTX = ();

    fn execute(&self, _ctx: &Self::CTX, input: &[&str]) -> anyhow::Result<()> {
        let mut game: GameTemplate = serde_json::from_str(
            fs::read_to_string(format!("{}{}.json", GAMES_DIR, input[0]))
                .unwrap()
                .as_str(),
        )
        .unwrap();
        let cards: Vec<CardTemplate> = game
            .card_paths
            .iter()
            .map(|path| serde_json::from_str(fs::read_to_string(path).unwrap().as_str()).unwrap())
            .collect::<Vec<_>>();
        game.cards = cards;
        let code_path = game.code_path.clone();
        // FIXME: enforce player limits
        let game = GameCtx {
            game,
            players: input
                .iter()
                .skip(1)
                .map(|player| PlayerDef {
                    name: player.to_string(),
                    inventories: Mutex::new(vec![]),
                    meta: Mutex::new(HashMap::new()),
                    active: true,
                })
                .collect::<Vec<_>>(),
            inventories: Mutex::new(vec![]),
            draw_stack: Mutex::new(vec![]),
            meta: HashMap::new(),
            curr_player: AtomicUsize::new(0),
        };
        CTX.store(Arc::new(game));
        // start game
        engine::run(
            &code_path,
            vec![
                Function {
                    params: &[],
                    var_len: false,
                    name: "nextPlayer",
                    call: next_player,
                },
                Function {
                    params: &[],
                    var_len: false,
                    name: "playerCount",
                    call: player_cnt,
                },
                Function {
                    params: &[],
                    var_len: true,
                    name: "playerName",
                    call: player_name,
                },
                Function {
                    params: &[],
                    var_len: true,
                    name: "createInvGlobal",
                    call: create_inv_global,
                },
                Function {
                    params: &[],
                    var_len: true,
                    name: "createInvRestricted",
                    call: create_inv_restricted,
                },
                Function {
                    params: &[],
                    var_len: true,
                    name: "storeMeta",
                    call: store_meta,
                },
                Function {
                    params: &[],
                    var_len: true,
                    name: "loadMeta",
                    call: load_meta,
                },
            ],
        )?;
        Ok(())
    }
}

const GAMES_DIR: &str = "./play_cards/games/";

struct CmdCreate;

impl CommandImpl for CmdCreate {
    type CTX = ();

    fn execute(&self, _ctx: &Self::CTX, input: &[&str]) -> anyhow::Result<()> {
        let game_name = input[0].to_string();
        let code_path = input[1].to_string();
        let min_players = input[2].parse::<usize>()?;
        let max_players = input[3].parse::<usize>()?;
        let cards = input
            .iter()
            .skip(4)
            .map(|path| path.to_string())
            .collect::<Vec<_>>();
        let out = serde_json::to_string(&GameTemplate {
            name: game_name,
            max_players,
            min_players,
            cards: vec![],
            card_paths: cards,
            code_path,
        })?;
        fs::write(format!("{}{}.json", GAMES_DIR, input[0]), out)?;
        CLI.get()
            .unwrap()
            .println(format!("Successfully created {}", input[0]).as_str());
        Ok(())
    }
}

struct CmdGames;

impl CommandImpl for CmdGames {
    type CTX = ();

    fn execute(&self, _ctx: &Self::CTX, _input: &[&str]) -> anyhow::Result<()> {
        let dir = fs::read_dir(GAMES_DIR)?;
        let mut games = vec![];
        for game in dir {
            games.push(game?.path());
        }
        CLI.get().unwrap().println(format!("Games ({}):", games.len()).as_str());
        for game_path in games {
            let game_name = game_path.file_name().unwrap().to_str().unwrap().to_string();
            let game: GameTemplate = serde_json::from_str(fs::read_to_string(game_path)?.as_str())?;
            CLI.get().unwrap().println(format!("{}: {:?}", game_name, game).as_str());
        }
        Ok(())
    }
}

const CARDS_DIR: &str = "./play_cards/cards/";

struct CmdCreateCard;

impl CommandImpl for CmdCreateCard {
    type CTX = ();

    fn execute(&self, _ctx: &Self::CTX, input: &[&str]) -> anyhow::Result<()> {
        let name = input[0].to_string();
        let ord = input[1].parse::<usize>()?;
        let image_path = input[2].to_string();
        fs::write(
            format!("{}{}.json", CARDS_DIR, name),
            serde_json::to_string_pretty(&CardTemplate {
                name,
                ord,
                image_path,
                image: Arc::new(DynamicImage::default()),
                metadata: HashMap::new(),
            })?,
        )?;
        CLI.get().unwrap().println(format!("Created card {}", input[0]).as_str());
        Ok(())
    }
}
