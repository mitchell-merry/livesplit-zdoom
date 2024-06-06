use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Formatter};
use asr::{future::next_tick, timer, watcher::Watcher, Error, Process, Address, settings};
use asr::settings::Gui;
use asr::settings::gui::Title;
use asr::string::ArrayCString;
use zdoom::{
    player::{DVector3, PlayerState},
    GameAction, ZDoom, ZDoomVersion,
};
use zdoom::pclass::PClass;
use zdoom::player::Player;
use zdoom::tarray::TArray;

asr::async_main!(stable);

#[derive(Gui)]
struct Settings {
    objectives: Title,
    /// Preparations
    #[heading_level = 1]
    _999: Title,
    /// Find a way past the soldiers and locate a weapon
    _4294966296: bool,
    /// Prepare to fight
    _4294966295: bool,
    /// Locate your combat suit from Personal Belongings
    _4294966294: bool,
    /// The Lockdown
    #[heading_level = 1]
    _995: Title,
    /// Head back to the Blue Door
    _4294966292: bool,
    /// Find a way to disengage the lockdown
    _4294966293: bool,
    #[heading_level = 1]
    /// Escape
    _3: Title,
    /// Find the exit.
    _4294966291: bool,
}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("SELACO.exe").await;
        process
            .until_closes(async {
                on_attach(&process, &mut settings).await.expect("problem");
            })
            .await;
    }
}

struct FoundClasses<'a> {
    objectives_class: PClass<'a>,
    objective_class: PClass<'a>,
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Option<Error>> {
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

        settings.update();
        let settings_map = asr::settings::Map::load();
        let val = if let Some(bal_v) = settings_map.get("bal") {
            bal_v.get_bool().unwrap_or_default()
        } else {
            false
        };

        asr::timer::set_variable("bal", &format!("{:?}", val));

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

        let settings_map = settings::Map::load();

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
            for (objective_key, old_objective_status) in old.objective_status {
                if let Some(current_objective_status) = current.objective_status.get(&objective_key) {
                    if old_objective_status == 0 && current_objective_status.to_owned() == 1 {
                        asr::print_message(&format!("completed {objective_key}"));

                        if safe_get_bool(&objective_key) {
                            asr::timer::split();
                        }
                    }
                }
            }
        }

        // if old.objective_history.len() < current.objective_history.len() && old.objective_history.len() != 0 {
        //     for completed_objective in current.objective_history {
        //         if !old.objective_history.contains(&completed_objective) {
        //             asr::print_message(&format!("Potentially completed {completed_objective}"));
        //         }
        //     }
        // }

        next_tick().await;
    }
}

fn safe_get_bool(key: &String) -> bool {
    let settings_map = settings::Map::load();

    settings_map.get(key).unwrap_or(settings::Value::from(false)).get_bool().unwrap_or_default()
}

struct AutoSplitterState {
    gameaction: GameAction,
    level: String,
    playerstate: PlayerState,
    player_pos: DVector3,
    objective_history: Vec<Objective>,
    objective_status: HashMap<String, u32>,
}

#[derive(Default)]
struct Watchers {
    gameaction: Watcher<GameAction>,
    level: Watcher<String>,
    playerstate: Watcher<PlayerState>,
    player_pos: Watcher<DVector3>,
    objective_history: Watcher<Vec<Objective>>,
    objective_status: Watcher<HashMap<String, u32>>,
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
        timer::set_variable("objectives", &format!("{:#?}", objectives));
        // timer::set_variable("history", &format!("{:#?}", objective_history));

        let mut map = HashMap::new();
        get_objective_status_map(&objectives, &mut map);
        get_objective_status_map(&objective_history, &mut map);

        let sorted_map: BTreeMap<_, _> = map.clone().into_iter().collect();
        // timer::set_variable("objective_status", &format!("{:#?}", sorted_map));

        self.objective_history.update(Some(objective_history));
        self.objective_status.update(Some(map));

        Ok(())
    }

    fn to_states(&self) -> Option<(AutoSplitterState, AutoSplitterState)> {
        let level = self.level.pair.as_ref()?;
        let player_pos = self.player_pos.pair.as_ref()?;
        let objectives = self.objective_history.pair.as_ref()?;
        let objective_status = self.objective_status.pair.as_ref()?;

        Some((
            AutoSplitterState {
                gameaction: self.gameaction.pair?.old,
                level: level.old.to_owned(),
                playerstate: self.playerstate.pair?.old,
                player_pos: player_pos.old.to_owned(),
                objective_history: objectives.old.to_owned(),
                objective_status: objective_status.old.to_owned(),
            },
            AutoSplitterState {
                gameaction: self.gameaction.pair?.current,
                level: level.current.to_owned(),
                playerstate: self.playerstate.pair?.current,
                player_pos: player_pos.current.to_owned(),
                objective_history: objectives.current.to_owned(),
                objective_status: objective_status.current.to_owned(),
            },
        ))
    }
}

fn get_objective_status_map(objectives: &Vec<Objective>, map: &mut HashMap<String, u32>) {
    for obj in objectives {
        map.insert(format!("_{}", obj.tag), obj.status);
        get_objective_status_map(&obj.children, map);
    }
}

#[derive(Clone, Debug)]
struct Objective {
    title: String,
    tag: u32,
    status: u32,
    children: Vec<Objective>,
}

impl Objective {
    pub fn read(process: &Process, address: Address, classes: &FoundClasses) -> Result<Self, Option<Error>> {
        let children_offset = classes.objective_class.fields()?.get("children").ok_or(None)?.offset()?.to_owned() as u64;
        let title_offset = classes.objective_class.fields()?.get("title").ok_or(None)?.offset()?.to_owned() as u64;
        let status_offset = classes.objective_class.fields()?.get("status").ok_or(None)?.offset()?.to_owned() as u64;
        let tag_offset = classes.objective_class.fields()?.get("tag").ok_or(None)?.offset()?.to_owned() as u64;

        let title = process.read_pointer_path::<ArrayCString<128>>(address, asr::PointerSize::Bit64, &[title_offset, 0x0])?.validate_utf8()
            .expect("title should always be utf-8")
            .to_owned();

        let tag = process.read(address + tag_offset)?;
        let status = process.read(address + status_offset)?;
        let children = read_objectives(process, address + children_offset, classes)?;

        Ok(Objective {
            title,
            tag,
            status,
            children,
        })
    }
}

fn read_objectives(process: &Process, address: Address, classes: &FoundClasses) -> Result<Vec<Objective>, Option<Error>> {
    let objectives_arr = TArray::<u64>::new(process, address);
    let mut objectives = Vec::new();
    for objective in objectives_arr.into_iter()? {
        if let Ok(obj) = Objective::read(process, objective.into(), classes) {
            objectives.push(obj);
        }
    }
    return Ok(objectives);
}

fn get_completed_objectives(process: &Process, zdoom: &ZDoom, classes: &FoundClasses) -> Result<(Vec<Objective>, Vec<Objective>), Option<Error>> {
    let objs_offset = classes.objectives_class.fields()?.get("objs").ok_or(None)?.offset()?.to_owned() as u64;
    let history_offset = classes.objectives_class.fields()?.get("history").ok_or(None)?.offset()?.to_owned() as u64;

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
            return Ok((read_objectives(process, inv + objs_offset, classes)?, read_objectives(process, inv + history_offset, classes)?));
        }
    }

    Ok((Vec::default(), Vec::default()))
}