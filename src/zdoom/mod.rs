use std::collections::HashMap;

use asr::{deep_pointer::DeepPointer, Address, Error, Process};

use self::{name_manager::NameManager, pclass::PClass, tarray::TArray};

pub mod name_manager;
pub mod pclass;
pub mod tarray;

pub struct ZDoom<'a> {
    process: &'a Process,
    pub name_data: NameManager<'a>,
    classes: HashMap<String, PClass<'a>>,
}

impl<'a> ZDoom<'a> {
    pub fn load(process: &'a Process, version: ZDoomVersion) -> Result<ZDoom<'a>, Error> {
        let memory = Memory::new(process, version)?;

        let name_data = NameManager::new(&process, memory.namedata_ptr.deref_offsets(process)?);

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
            name_data,
            classes,
        })
    }

    pub fn find_class(&self, name: &str) -> Option<&PClass> {
        self.classes.get(name)
    }

    pub fn show_all_classes(&self) {
        for (name, _class) in self.classes.iter() {
            asr::print_message(name);
        }
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
    namedata_ptr: DeepPointer<1>,
    // player_actor_class_ptr: DeepPointer<3>,
    all_classes_ptr: DeepPointer<1>,
}

impl Memory {
    fn new(process: &Process, version: ZDoomVersion) -> Result<Memory, Error> {
        let main_module_name = Memory::get_main_module_name(version);
        let main_exe_addr = process.get_module_address(main_module_name)?;

        match version {
            ZDoomVersion::Lzdoom3_82 => Ok(Memory {
                namedata_ptr: DeepPointer::new(main_exe_addr, asr::PointerSize::Bit64, &[0x9F8E10]),
                // player_actor_class_ptr: DeepPointer::new(
                //     main_exe_addr,
                //     asr::PointerSize::Bit64,
                //     &[0x7043C0, 0x0, 0x8],
                // ),
                all_classes_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x9F8980],
                ),
            }),
            ZDoomVersion::Gzdoom4_8_2 => Ok(Memory {
                namedata_ptr: DeepPointer::new(main_exe_addr, asr::PointerSize::Bit64, &[0x11880A0]),
                // player_actor_class_ptr: DeepPointer::new(
                //     main_exe_addr,
                //     asr::PointerSize::Bit64,
                //     &[0x6FDBD0, 0x0, 0x8],
                // ),
                all_classes_ptr: DeepPointer::new(
                    main_exe_addr,
                    asr::PointerSize::Bit64,
                    &[0x11147C0],
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
