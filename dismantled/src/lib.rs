use asr::{future::next_tick, timer, watcher::Watcher, Error, Process};
use zdoom::{
    player::{DVector3, PlayerState},
    GameAction, ZDoom, ZDoomVersion,
};

asr::async_main!(stable);

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
    let mut zdoom = ZDoom::load(process, ZDoomVersion::Lzdoom3_82, "lzdoom.exe").expect("");
    // zdoom.dump();
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

        let states = watchers.to_states();
        if states.is_none() {
            asr::print_message("some watcher is empty");
            continue;
        }

        let (old, current) = states.unwrap();

        if timer::state() == timer::TimerState::NotRunning
            && current.level == "MAP01"
            && current.player_pos.x == -22371.0
            && current.player_pos.y == 12672.0
            && old.gameaction == GameAction::WorldDone
            && current.gameaction == GameAction::Nothing
        {
            timer::start();
        }

        if timer::state() == timer::TimerState::Running {
            match current.gameaction {
                GameAction::WorldDone => timer::pause_game_time(),
                _ => timer::resume_game_time(),
            }
        }

        next_tick().await;
    }
}

struct AutoSplitterState {
    gameaction: GameAction,
    level: String,
    playerstate: PlayerState,
    player_pos: DVector3,
}

#[derive(Default)]
struct Watchers {
    gameaction: Watcher<GameAction>,
    level: Watcher<String>,
    playerstate: Watcher<PlayerState>,
    player_pos: Watcher<DVector3>,
}

impl Watchers {
    fn update(&mut self, _process: &Process, zdoom: &mut ZDoom) -> Result<(), Option<Error>> {
        zdoom.invalidate_cache().expect("");

        let gameaction = zdoom.gameaction().unwrap_or_default();
        timer::set_variable("gameaction", &format!("{:?}", gameaction));
        self.gameaction.update(Some(gameaction));

        let level_name = zdoom.level.name().map(|s| s.to_owned()).unwrap_or_default();
        timer::set_variable("map", level_name.as_str());
        self.level.update(Some(level_name));

        let player = zdoom.player()?;
        let playerstate = player.state()?;
        timer::set_variable("playerstate", &format!("{:?}", playerstate));
        self.playerstate.update(Some(playerstate));

        let player_pos = player.pos().map(|v| v.to_owned()).unwrap_or_default();
        timer::set_variable("pos", &format!("{:?}", player_pos));
        self.player_pos.update(Some(player_pos));

        Ok(())
    }

    fn to_states(&self) -> Option<(AutoSplitterState, AutoSplitterState)> {
        let level = self.level.pair.as_ref()?;
        let player_pos = self.player_pos.pair.as_ref()?;

        Some((
            AutoSplitterState {
                gameaction: self.gameaction.pair?.old,
                level: level.old.to_owned(),
                playerstate: self.playerstate.pair?.old,
                player_pos: player_pos.old.to_owned(),
            },
            AutoSplitterState {
                gameaction: self.gameaction.pair?.current,
                level: level.current.to_owned(),
                playerstate: self.playerstate.pair?.current,
                player_pos: player_pos.current.to_owned(),
            },
        ))
    }
}
