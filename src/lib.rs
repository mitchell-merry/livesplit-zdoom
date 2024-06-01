#![feature(type_alias_impl_trait, const_async_blocks, let_chains)]

mod zdoom;

use asr::{future::next_tick, timer, watcher::Watcher, Error, Process};
use zdoom::{player::DVector3, GameAction, ZDoom, ZDoomVersion};

asr::async_main!(nightly);

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("lzdoom.exe").await;
        process
            .until_closes(async {
                on_attach(&process).await.expect("problem");
            })
            .await;
    }
}

async fn on_attach(process: &Process) -> Result<(), Error> {
    let mut zdoom = ZDoom::load(process, ZDoomVersion::Lzdoom3_82).expect("");
    zdoom.dump();
    let mut watchers = Watchers::default();

    loop {
        if !process.is_open() {
            asr::print_message("process not open");
            return Ok(());
        }

        let res = watchers.update(process, &mut zdoom);
        if res.is_err() {
            asr::print_message("failed updating watchers");
            continue;
        }

        // this is logic specific to Dismantled
        if let Some(ref level_name) = watchers.level.pair
            && let Some(ref player_pos) = watchers.player_pos.pair
            && let Some(ref gameaction) = watchers.gameaction.pair
        {
            if timer::state() == timer::TimerState::NotRunning
                && level_name.current == "MAP01"
                && player_pos.current.x == -22371.0
                && player_pos.current.y == 12672.0
                && gameaction.old == GameAction::WorldDone
                && gameaction.current == GameAction::Nothing
            {
                timer::start();
            }

            if timer::state() == timer::TimerState::Running {
                match gameaction.current {
                    GameAction::WorldDone => timer::pause_game_time(),
                    _ => timer::resume_game_time(),
                }
            }
        }

        // this is logic specific to Snap the Sentinel
        // if let Some(ref level_name) = watchers.level.pair
        //     && let Some(ref player_pos) = watchers.player_pos.pair
        //     && let Some(ref gameaction) = watchers.gameaction.pair
        // {
        //     if timer::state() == timer::TimerState::NotRunning {
        //         if level_name.current == "E1M1"
        //             && player_pos.current.x == 64.0
        //             && player_pos.current.y == -848.0
        //             && gameaction.old == GameAction::NewGame
        //             && gameaction.current == GameAction::Nothing
        //         {
        //             timer::start();
        //         }
        //     }

        //     if timer::state() == timer::TimerState::Running {
        //         match gameaction.current {
        //             GameAction::WorldDone => timer::pause_game_time(),
        //             _ => timer::resume_game_time(),
        //         }
        //     }
        // }

        next_tick().await;
    }
}

#[derive(Default)]
struct Watchers {
    level: Watcher<String>,
    player_pos: Watcher<DVector3>,
    gameaction: Watcher<GameAction>,
}

impl Watchers {
    fn update(&mut self, _process: &Process, zdoom: &mut ZDoom) -> Result<(), Error> {
        zdoom.invalidate_cache().expect("");

        let level_name = zdoom.level.name().map(|s| s.to_owned()).unwrap_or_default();
        timer::set_variable("map", level_name.as_str());
        self.level.update(Some(level_name));

        let player_pos = zdoom
            .player()?
            .pos()
            .map(|v| v.to_owned())
            .unwrap_or_default();
        timer::set_variable("pos", &format!("{:?}", player_pos));
        self.player_pos.update(Some(player_pos));

        let gameaction = zdoom.gameaction().unwrap_or_default();
        timer::set_variable("gameaction", &format!("{:?}", gameaction));
        self.gameaction.update(Some(gameaction));

        Ok(())
    }
}
