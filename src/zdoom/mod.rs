use std::collections::HashMap;

use asr::{deep_pointer::DeepPointer, Address, Error, Process};
use bytemuck::CheckedBitPattern;

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
    memory: Memory,
    pub name_data: NameManager<'a>,
    classes: HashMap<String, PClass<'a>>,
    pub level: Level<'a>,
    pub player: Player<'a>,
    pub gameaction: Option<GameAction>,
}

impl<'a> ZDoom<'a> {
    pub fn load(process: &'a Process, version: ZDoomVersion) -> Result<ZDoom<'a>, Error> {
        let memory = Memory::new(process, version)?;

        let name_data = NameManager::new(&process, memory.namedata_ptr.deref_offsets(process)?);
        let level = Level::new(&process, memory.level_ptr.deref_offsets(process)?);
        let player = Player::new(&process, memory.player_ptr.deref::<u64>(process)?.into());

        let mut classes: HashMap<String, PClass<'a>> = HashMap::new();
        let all_classes =
            TArray::<u64>::new(process, memory.all_classes_ptr.deref_offsets(process)?);

        for class in all_classes.into_iter()? {
            let pclass = PClass::<'a>::new(process, class.into());
            let name = pclass.name(&name_data)?;

            classes.insert(name, pclass);
        }

        Ok(ZDoom {
            process,
            memory,
            name_data,
            classes,
            level,
            player,
            gameaction: None,
        })
    }

    pub fn invalidate_cache(&mut self) -> Result<(), Error> {
        self.level.invalidate_cache();
        self.player = Player::new(
            self.process,
            self.memory.player_ptr.deref::<u64>(self.process)?.into(),
        );
        self.gameaction = None;

        Ok(())
    }

    pub fn find_class(&self, name: &str) -> Option<&PClass> {
        self.classes.get(name)
    }

    pub fn show_all_classes(&self) {
        for (name, _class) in self.classes.iter() {
            asr::print_message(name);
        }
    }

    pub fn gameaction(&mut self) -> Result<GameAction, Error> {
        if let Some(gameaction) = self.gameaction {
            return Ok(gameaction);
        }

        let gameaction = self.memory.gameaction_ptr.deref(self.process)?;

        self.gameaction = Some(gameaction);
        return Ok(gameaction);
    }
}

// disclaimer: I don't know much about the different zdoom versions work...
// i have only tried this with two games
#[derive(Clone, Copy)]
pub enum ZDoomVersion {
    Lzdoom3_82,  // Dismantled: Director's Cut
    Gzdoom4_8_2, // Snap the Sentinel
}

struct Memory {
    // yes these should be signatures. TODO
    namedata_ptr: DeepPointer<1>,
    player_ptr: DeepPointer<2>,
    all_classes_ptr: DeepPointer<1>,
    level_ptr: DeepPointer<1>,
    gameaction_ptr: DeepPointer<2>,
}

impl Memory {
    fn new(process: &Process, version: ZDoomVersion) -> Result<Memory, Error> {
        let main_module_name = Memory::get_main_module_name(version);
        let main_exe_addr = process.get_module_address(main_module_name)?;

        match version {
            ZDoomVersion::Lzdoom3_82 => Ok(Memory {
                namedata_ptr: DeepPointer::new(main_exe_addr, asr::PointerSize::Bit64, &[0x9F8E10]),
                player_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x7043C0, 0x0],
                ),
                all_classes_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x9F8980],
                ),
                level_ptr: DeepPointer::new(main_exe_addr, asr::PointerSize::Bit64, &[0x9F5B78]),
                gameaction_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x7044E0, 0],
                ),
            }),
            ZDoomVersion::Gzdoom4_8_2 => Ok(Memory {
                namedata_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x11880A0],
                ),
                player_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x6FDBD0, 0x0],
                ),
                all_classes_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x11147C0],
                ),
                level_ptr: DeepPointer::new(main_exe_addr, asr::PointerSize::Bit64, &[0x10FD9B0]),
                gameaction_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x6FDCF0, 0],
                ),
            }),
        }
    }

    fn get_main_module_name(version: ZDoomVersion) -> &'static str {
        match version {
            ZDoomVersion::Lzdoom3_82 => "lzdoom.exe",
            ZDoomVersion::Gzdoom4_8_2 => "gzdoom.exe",
        }
    }
}

#[derive(CheckedBitPattern, Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum GameAction {
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
