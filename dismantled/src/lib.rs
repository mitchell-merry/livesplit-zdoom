use asr::settings::gui::Title;
use asr::settings::Gui;
use asr::{future::next_tick, settings, timer, watcher::Watcher, Error, Process};
use std::collections::HashSet;
use zdoom::pclass::PClass;
use zdoom::{
    player::{DVector3, PlayerState},
    GameAction, ZDoom, ZDoomVersion,
};

asr::async_main!(stable);

const TRUE_ENDING_POSITION: DVector3 = DVector3 {
    x: -15032.0,
    y: 9752.0,
    z: -632.0,
};

#[derive(Gui)]
struct Settings {
    /// Split on run end
    #[default = true]
    split_run_end: bool,
    /// Split on entering level
    #[heading_level = 0]
    split_entering_level: Title,
    /// Mansion Upstairs
    _level_map02: bool,
    /// Mansion Attic
    _level_map03: bool,
    /// Underground (cave outside mansion)
    _level_map05: bool,
    /// Even more underground (where the gas mask is)
    _level_map08: bool,
    /// Toxic Room
    _level_map06: bool,
    /// Second part of the outside
    _level_map10: bool,
    /// Outside the Cemetery (leaving the cemetery out the back)
    _level_map07: bool,
    /// Split on item pickup
    #[heading_level = 0]
    split_item_pickup: Title,
    /// Story Items
    #[heading_level = 1]
    split_item_pickup_story: Title,
    /// Torture Room Key (basement 3)
    _item_keybasement3: bool,
    /// The next key (basement 2)
    _item_keybasement2: bool,
    /// Bolt Cutter
    _item_boltcutter: bool,
    /// Another basement key (basement 1)
    _item_keybasement1: bool,
    /// Key just after the timer puzzle (11)
    _item_keyf11: bool,
    /// Key just after the key just after the timer puzzle (12)
    _item_keyf12: bool,
    /// Key just after the key just after the key just after the timer puzzle (13)
    _item_keyf13: bool,
    /// Key in the library (22)
    _item_keyf22: bool,
    /// Bomb
    _item_plasticbomb: bool,
    /// Key from statue puzzle (21)
    _item_keyf21: bool,
    /// Yellow Cable (idk on a table)
    _item_yellowcable: bool,
    /// Rooftop Key (14)
    _item_keyf14: bool,
    /// Red Cable (in cage puzzle)
    _item_redcable: bool,
    /// Ruby (in outside statue)
    _item_ruby: bool,
    /// Emerald (in party)
    _item_emerald: bool,
    /// Crank
    _item_squarecrank: bool,
    /// Gas Mask
    _item_gazmask: bool,
    /// Topaz (in gas mask area)
    _item_topaz: bool,
    /// Cemetery Key
    _item_keycemetery: bool,
    /// Shovel
    _item_shovel: bool,
    /// Statue Head
    _item_helenahead: bool,
    /// Coin (from statue)
    _item_dm_coin: bool,
    /// Key from cemetery attic (H3)
    _item_dm_keyfh3: bool,
    /// Key from cemetery morgue (H2)
    _item_dm_keyfh2: bool,
    /// Weapons
    #[heading_level = 1]
    split_item_pickup_weapons: Title,
    /// Fireaxe
    _item_fireaxe: bool,
    /// Beretta
    _item_beretta: bool,
    /// Shotgun
    _item_dm_shotgun: bool,
    /// Annihilator
    _item_annihilator: bool,
}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("lzdoom.exe").await;
        process
            .until_closes(async {
                on_attach(&process, &mut settings).await.expect("problem");
            })
            .await;
    }
}

async fn on_attach(process: &Process, settings: &mut Settings) -> Result<(), Option<Error>> {
    let (mut zdoom, _) =
        ZDoom::wait_try_load(process, ZDoomVersion::Lzdoom3_82, "lzdoom.exe", |_| Ok(())).await;

    // zdoom.dump();

    let mut watchers = Watchers::default();
    let mut completed_splits = HashSet::new();

    loop {
        if !process.is_open() {
            asr::print_message("process not open");
            return Ok(());
        }

        settings.update();

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
            && old.level == "map45"
            && current.level == "MAP01"
        {
            timer::start();
        }

        if timer::state() == timer::TimerState::Running {
            match current.gameaction {
                GameAction::WorldDone => timer::pause_game_time(),
                _ => timer::resume_game_time(),
            }

            if old.level != current.level {
                let key = &format!("_level_{}", current.level.to_lowercase());
                if safe_get_bool(key, &mut completed_splits) {
                    timer::split();
                }
            }

            if old.inventories.len() != 0 {
                for inventory in current.inventories {
                    if !old.inventories.contains(&inventory) {
                        asr::print_message(&format!("Picked up {inventory}"));
                        let key = &format!("_item_{}", inventory.to_owned().to_lowercase());
                        if safe_get_bool(key, &mut completed_splits) {
                            timer::split();
                        }
                    }
                }
            }

            if current.level == "MAP07"
                && current.player_pos == TRUE_ENDING_POSITION
                && old.player_pos != current.player_pos
                && safe_get_bool(&String::from("split_run_end"), &mut completed_splits)
            {
                timer::split();
            }
        }

        next_tick().await;
    }
}

fn safe_get_bool(key: &String, completed_splits: &mut HashSet<String>) -> bool {
    let settings_map = settings::Map::load();

    if completed_splits.contains(key) {
        return false;
    }

    return if settings_map
        .get(key)
        .unwrap_or(settings::Value::from(false))
        .get_bool()
        .unwrap_or_default()
    {
        completed_splits.insert(key.to_owned());
        true
    } else {
        false
    };
}

struct AutoSplitterState {
    gameaction: GameAction,
    level: String,
    playerstate: PlayerState,
    player_pos: DVector3,
    inventories: HashSet<String>,
}

#[derive(Default)]
struct Watchers {
    gameaction: Watcher<GameAction>,
    level: Watcher<String>,
    playerstate: Watcher<PlayerState>,
    player_pos: Watcher<DVector3>,
    inventories: Watcher<HashSet<String>>,
}

impl Watchers {
    fn update(&mut self, process: &Process, zdoom: &mut ZDoom) -> Result<(), Option<Error>> {
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

        let mut inventories = HashSet::new();
        let invs = zdoom.player()?.get_inventories().unwrap_or_default();

        for inv in invs {
            let class = process.read::<u64>(inv + 0x8)?.into();
            let class = PClass::new(
                process,
                zdoom.memory.clone(),
                zdoom.name_data.clone(),
                class,
            );

            let name = class.name()?;
            inventories.insert(name.to_owned());
        }
        let mut vec = Vec::from_iter(inventories.clone().into_iter());
        vec.sort();
        timer::set_variable("inventories", &format!("{:#?}", vec));
        self.inventories.update(Some(inventories));

        Ok(())
    }

    fn to_states(&self) -> Option<(AutoSplitterState, AutoSplitterState)> {
        let level = self.level.pair.as_ref()?;
        let player_pos = self.player_pos.pair.as_ref()?;
        let inventories = self.inventories.pair.as_ref()?;

        Some((
            AutoSplitterState {
                gameaction: self.gameaction.pair?.old,
                level: level.old.to_owned(),
                playerstate: self.playerstate.pair?.old,
                player_pos: player_pos.old.to_owned(),
                inventories: inventories.old.to_owned(),
            },
            AutoSplitterState {
                gameaction: self.gameaction.pair?.current,
                level: level.current.to_owned(),
                playerstate: self.playerstate.pair?.current,
                player_pos: player_pos.current.to_owned(),
                inventories: inventories.current.to_owned(),
            },
        ))
    }
}
