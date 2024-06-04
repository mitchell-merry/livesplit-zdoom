use std::time::{Duration, Instant};
use std::{collections::HashMap, rc::Rc};

use asr::future::{next_tick, retry};
use asr::{deep_pointer::DeepPointer, signature::Signature, Address, Error, Process};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;

use self::{
    level::Level, name_manager::NameManager, pclass::PClass, player::Player, tarray::TArray,
};

pub mod level;
pub mod name_manager;
pub mod pclass;
pub mod player;
pub mod tarray;

pub struct ZDoom<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    name_data: Rc<NameManager<'a>>,
    classes: OnceCell<HashMap<String, PClass<'a>>>,
    actor_class: OnceCell<PClass<'a>>,

    pub level: Level<'a>,
    pub player: OnceCell<Player<'a>>,
    pub gameaction: OnceCell<GameAction>,
}

impl<'a> ZDoom<'a> {
    pub async fn wait_try_load<T, F>(
        process: &'a Process,
        version: ZDoomVersion,
        main_module_name: &str,
        load_fn: F,
    ) -> (ZDoom<'a>, T)
    where
        F: Fn(&HashMap<String, PClass<'a>>) -> Result<T, Option<Error>>,
    {
        let cooldown = Duration::from_secs(3);

        let fail_action = || async {
            asr::print_message(&format!(
                "try_load unsuccessful, waiting {}s...",
                cooldown.as_secs()
            ));
            asr::future::sleep(cooldown).await;
        };

        loop {
            let memory = Memory::new(process, version, main_module_name);
            if memory.is_err() {
                fail_action().await;
                continue
            }
            let memory = Rc::new(memory.unwrap());

            let name_data = Rc::new(NameManager::new(process, memory.namedata_addr));
            let level = Level::new(process, memory.clone(), memory.level_addr);

            let zdoom = ZDoom {
                process,
                memory,
                name_data,
                level,
                classes: OnceCell::new(),
                actor_class: OnceCell::new(),
                player: OnceCell::new(),
                gameaction: OnceCell::new(),
            };

            let classes = zdoom.classes();
            // assert that we have the Actor class, we need it for Player shenanigans
            if classes.is_err() {
                fail_action().await;
                continue;
            }

            let classes = classes.unwrap();
            if !classes.contains_key("Actor") {
                fail_action().await;
                continue;
            }

            let result = load_fn(&classes);

            if result.is_err() {
                fail_action().await;
                continue;
            }

            asr::print_message("try_load successful!");
            return (zdoom, result.unwrap());
        }
    }

    pub fn classes(&self) -> Result<&HashMap<String, PClass<'a>>, Error> {
        self.classes.get_or_try_init(|| {
            let mut classes: HashMap<String, PClass<'a>> = HashMap::new();
            let all_classes = TArray::<u64>::new(self.process, self.memory.all_classes_addr);

            for class in all_classes.into_iter()? {
                let pclass = PClass::<'a>::new(
                    self.process,
                    self.memory.clone(),
                    self.name_data.clone(),
                    class.into(),
                );
                let name = pclass.name()?.to_owned();

                classes.insert(name, pclass);
            }

            Ok(classes)
        })
    }

    pub fn invalidate_cache(&mut self) -> Result<(), Error> {
        self.level.invalidate_cache();
        self.player = OnceCell::new();
        self.gameaction = OnceCell::new();

        Ok(())
    }

    pub fn find_class(&self, name: &str) -> Result<Option<&PClass<'a>>, Error> {
        Ok(self.classes()?.get(name))
    }

    pub fn dump(&self) -> Result<(), Error> {
        for (name, class) in self.classes()?.iter() {
            let c = class
                .show_class()
                .unwrap_or(format!("// failed getting {name}"));
            asr::print_message(&format!("{c}\n"));
        }

        Ok(())
    }

    pub fn player<'b>(&'b self) -> Result<&'b Player<'a>, Option<Error>> {
        self.player.get_or_try_init(|| {
            let actor_class = self
                .classes()?
                .get("Actor")
                .expect("we should have asserted the Actor class was found")
                .to_owned();

            Ok(Player::new(
                self.process,
                self.memory.clone(),
                self.name_data.clone(),
                // 0x0 is the first index
                self.memory.players_addr + 0x0,
                actor_class,
            ))
        })
    }

    pub fn gameaction(&self) -> Result<GameAction, Error> {
        self.gameaction
            .get_or_try_init(|| self.process.read(self.memory.gameaction_addr))
            .map(|v| v.to_owned())
    }
}

// disclaimer: I don't know much about the different zdoom versions work...
// i have only tried this with a few games
#[derive(Clone, Copy)]
pub enum ZDoomVersion {
    Lzdoom3_82,   // Dismantled: Director's Cut
    Gzdoom4_8Pre, // Selaco
    Gzdoom4_8_2,  // Snap the Sentinel
}

pub struct Memory {
    namedata_addr: Address,
    players_addr: Address,
    all_classes_addr: Address,
    level_addr: Address,
    gameaction_addr: Address,

    offsets: Offsets,
}

impl Memory {
    fn new(
        process: &Process,
        version: ZDoomVersion,
        main_module_name: &str,
    ) -> Result<Memory, Error> {
        let main_exe_addr = process.get_module_address(main_module_name)?;
        let module_range = process.get_module_range(main_module_name)?;

        match version {
            // yes these should be signatures or something. TODO
            ZDoomVersion::Lzdoom3_82 => Ok(Memory {
                namedata_addr: main_exe_addr + 0x9F8E10,
                players_addr: main_exe_addr + 0x9F3CD0,
                all_classes_addr: main_exe_addr + 0x9F8980,
                level_addr: main_exe_addr + 0x9F5B78,
                gameaction_addr: main_exe_addr + 0x7044E0,
                offsets: Offsets::new(version),
            }),
            ZDoomVersion::Gzdoom4_8Pre | ZDoomVersion::Gzdoom4_8_2 => {
                let s = Signature::<23>::new(
                    "45 33 C0 48 8B D6 48 8D 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 44 8B C0 8B 15",
                );
                let namedata_addr = scan_rel(process, module_range, &s, 0x9, 0x4)?;

                let s = Signature::<33>::new("B2 01 89 05 ?? ?? ?? ?? E8 ?? ?? ?? ?? C7 05 ?? ?? ?? ?? 03 00 00 00 C7 05 ?? ?? ?? ?? 02 00 00 00");
                let gameaction_addr = scan_rel(process, module_range, &s, 0xF, 0x8)?;

                let s = Signature::<13>::new("48 8B 05 ?? ?? ?? ?? 48 39 03 75 09 E8");
                let level_addr: Address = process
                    .read::<u64>(scan_rel(process, module_range, &s, 0x3, 0x4)?)?
                    .into();

                let s = Signature::<11>::new("48 8B 84 29 ?? ?? ?? ?? 48 85 C0");
                let players_addr_offset = process.read::<u32>(
                    s.scan_process_range(process, module_range)
                        .unwrap_or_else(|| panic!("failed to get address"))
                        + 0x4,
                )?;

                let s = Signature::<17>::new("48 8B 05 ?? ?? ?? ?? 48 8B 1C F0 48 8B C3 48 85 DB");
                let all_classes_addr = scan_rel(process, module_range, &s, 0x3, 0x4)?;

                Ok(Memory {
                    namedata_addr,
                    players_addr: main_exe_addr + players_addr_offset,
                    all_classes_addr,
                    level_addr,
                    gameaction_addr,
                    offsets: Offsets::new(version),
                })
            }
        }
    }
}

struct Offsets {
    pclass_fields: u64,
    level_mapname: u64,
}

impl Offsets {
    fn new(version: ZDoomVersion) -> Self {
        match version {
            ZDoomVersion::Lzdoom3_82 => Self {
                pclass_fields: 0x78,
                level_mapname: 0x2C8,
            },
            ZDoomVersion::Gzdoom4_8Pre => Self {
                pclass_fields: 0x80,
                level_mapname: 0x9F8,
            },
            ZDoomVersion::Gzdoom4_8_2 => Self {
                pclass_fields: 0x78,
                level_mapname: 0x9D8,
            },
        }
    }
}

#[derive(CheckedBitPattern, Clone, Copy, Debug, Default, PartialEq)]
#[repr(u32)]
pub enum GameAction {
    #[default]
    Nothing,
    LoadLevel, // not used.
    NewGame,
    NewGame2,
    RecordGame,
    LoadGame,
    LoadGameHideCon,
    LoadGamePlayDemo,
    AutoLoadGame,
    SaveGame,
    AutoSave,
    PlayDemo,
    Completed,
    Slideshow,
    WorldDone,
    Screenshot,
    ToggleMap,
    FullConsole,
    ResumeConversation,
    Intro,
    Intermission,
    TitleLoop,
}

fn scan_rel<const N: usize>(
    process: &Process,
    module_range: (Address, u64),
    signature: &Signature<N>,
    offset: u32,
    next_instruction: u32,
) -> Result<Address, Error> {
    let addr = signature
        .scan_process_range(process, module_range)
        .unwrap_or_else(|| panic!("failed to get address"))
        + offset;

    Ok(addr + process.read::<u32>(addr)? + next_instruction)
}
