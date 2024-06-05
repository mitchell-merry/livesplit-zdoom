use std::fmt::Debug;
use asr::{future::next_tick, timer, watcher::Watcher, Error, Process, Address};
use asr::string::ArrayCString;
use zdoom::{
    player::{DVector3, PlayerState},
    GameAction, ZDoom, ZDoomVersion,
};
use zdoom::pclass::PClass;
use zdoom::player::Player;
use zdoom::tarray::TArray;

asr::async_main!(stable);

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("SELACO.exe").await;
        process
            .until_closes(async {
                on_attach(&process).await.expect("problem");
            })
            .await;
    }
}

struct FoundClasses<'a> {
    objectives_class: PClass<'a>,
    objective_class: PClass<'a>,
}

async fn on_attach(process: &Process) -> Result<(), Option<Error>> {
    let (mut zdoom, classes) = ZDoom::wait_try_load(
        process,
        ZDoomVersion::Gzdoom4_8Pre,
        "Selaco.exe",
        |classes| {
            let objectives_class = classes.get("Objectives").ok_or(None)?.to_owned();
            let objective_class = classes.get("Objective").ok_or(None)?.to_owned();

            Ok(FoundClasses {
                objectives_class
            , objective_class})
        },
    )
    .await;
    // let _ = zdoom.dump();
    // if let Ok(player) = zdoom.player() {
    //     let _ = player.dump_inventories();
    // }

    let mut watchers = Watchers::default();

    loop {
        if !process.is_open() {
            asr::print_message("process not open");
            return Ok(());
        }

        let res = watchers.update(process, &mut zdoom, &classes);
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

        if timer::state() == timer::TimerState::NotRunning {
            if current.level == "SE_01a"
                && old.gameaction == GameAction::NewGame
                && current.gameaction != GameAction::NewGame
            {
                timer::start();
            }
        }

        if timer::state() == timer::TimerState::Running {
            // isLoading
            if old.gameaction == GameAction::Nothing && current.gameaction == GameAction::Completed
                || old.playerstate == PlayerState::Dead && current.playerstate == PlayerState::Enter
            {
                timer::pause_game_time();
            }

            if old.gameaction == GameAction::WorldDone && current.gameaction == GameAction::Nothing
                || old.playerstate == PlayerState::Enter && current.playerstate == PlayerState::Live
            {
                timer::resume_game_time();
            }

            // split
            if old.objective_history.len() < current.objective_history.len() && old.objective_history.len() != 0 {
                for completed_objective in current.objective_history {
                    if !old.objective_history.contains(&completed_objective) {
                        asr::print_message(&format!("Potentially completed {completed_objective}"));
                    }
                }
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
    objective_history: Vec<String>,
}

#[derive(Default)]
struct Watchers {
    gameaction: Watcher<GameAction>,
    level: Watcher<String>,
    playerstate: Watcher<PlayerState>,
    player_pos: Watcher<DVector3>,
    objective_history: Watcher<Vec<String>>,
}

impl Watchers {
    fn update(&mut self, process: &Process, zdoom: &mut ZDoom, classes: &FoundClasses) -> Result<(), Option<Error>> {
        zdoom.invalidate_cache().expect("");

        let gameaction = zdoom.gameaction().unwrap_or_default();
        timer::set_variable("gameaction", &format!("{:?}", gameaction));
        self.gameaction.update(Some(gameaction));

        let level_name = zdoom.level.name().map(|s| s.to_owned()).unwrap_or_default();
        timer::set_variable("map", level_name.as_str());
        self.level.update(Some(level_name));

        let player = zdoom.player()?;
        let playerstate = player.state()?.to_owned();
        timer::set_variable("playerstate", &format!("{:?}", playerstate));
        self.playerstate.update(Some(playerstate));

        let player_pos = player.pos().map(|v| v.to_owned()).unwrap_or_default();
        timer::set_variable("pos", &format!("{:?}", player_pos));
        self.player_pos.update(Some(player_pos));

        let (objectives, objective_history) = get_completed_objectives(process, zdoom, classes).unwrap_or_default();
        timer::set_variable("objectives", &format!("{:?}", objectives));
        timer::set_variable("history", &format!("{:?}", objective_history));
        self.objective_history.update(Some(objective_history));

        Ok(())
    }

    fn to_states(&self) -> Option<(AutoSplitterState, AutoSplitterState)> {
        let level = self.level.pair.as_ref()?;
        let player_pos = self.player_pos.pair.as_ref()?;
        let objectives = self.objective_history.pair.as_ref()?;

        Some((
            AutoSplitterState {
                gameaction: self.gameaction.pair?.old,
                level: level.old.to_owned(),
                playerstate: self.playerstate.pair?.old,
                player_pos: player_pos.old.to_owned(),
                objective_history: objectives.old.to_owned(),
            },
            AutoSplitterState {
                gameaction: self.gameaction.pair?.current,
                level: level.current.to_owned(),
                playerstate: self.playerstate.pair?.current,
                player_pos: player_pos.current.to_owned(),
                objective_history: objectives.current.to_owned(),
            },
        ))
    }
}

fn get_completed_objectives(process: &Process, zdoom: &ZDoom, classes: &FoundClasses) -> Result<(Vec<String>, Vec<String>), Option<Error>> {
    let objs_offset = classes.objectives_class.fields()?.get("objs").ok_or(None)?.offset()?.to_owned() as u64;
    let history_offset = classes.objectives_class.fields()?.get("history").ok_or(None)?.offset()?.to_owned() as u64;
    let title_offset = classes.objective_class.fields()?.get("title").ok_or(None)?.offset()?.to_owned() as u64;

    let player = zdoom.player()?;
    let player_inventories = player.get_inventories()?;

    for inv in player_inventories {
        let class = process.read::<u64>(inv + 0x8)?.into();
        let class = PClass::new(
            process,
            zdoom.memory.clone(),
            zdoom.name_data.clone(),
            class,
        );

        let name = class.name()?;
        if name == "Objectives" {
            let objectives_arr = TArray::<u64>::new(process, inv + objs_offset);
            let mut objectives = Vec::new();
            for objective in objectives_arr.into_iter()? {
                let title = process.read_pointer_path::<ArrayCString<128>>(Address::from(objective), asr::PointerSize::Bit64, &[title_offset, 0x0])?.validate_utf8()
                    .expect("title should always be utf-8")
                    .to_owned();

                objectives.push(title);
            }

            let mut objectives_history = Vec::new();
            let objectives_arr = TArray::<u64>::new(process, inv + history_offset);
            for objective in objectives_arr.into_iter()? {
                let title = process.read_pointer_path::<ArrayCString<128>>(Address::from(objective), asr::PointerSize::Bit64, &[title_offset, 0x0])?.validate_utf8()
                    .expect("title should always be utf-8")
                    .to_owned();

                objectives_history.push(title);
            }

            return Ok((objectives, objectives_history));
        }
    }

    Ok((Vec::default(), Vec::default()))
}