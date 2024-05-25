use std::collections::HashMap;

use asr::{deep_pointer::DeepPointer, Address, Error, Process};

use self::{name_manager::NameManager, pclass::PClass, tarray::TArray};

pub mod name_manager;
pub mod pclass;
pub mod tarray;

pub struct ZDoom<'a> {
    process: &'a Process,
    name_data: NameManager<'a>,
    classes: HashMap<String, PClass<'a>>,
}

impl<'a> ZDoom<'a> {
    pub fn load(process: &'a Process) -> Result<ZDoom<'a>, Error> {
        let main_exe_addr = process.get_module_address("lzdoom.exe")?;
        let memory = Memory::new(main_exe_addr);

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

struct Memory {
    namedata_ptr: DeepPointer<1>,
    player_actor_class_ptr: DeepPointer<3>,
    all_classes_ptr: DeepPointer<1>,
}

impl Memory {
    fn new(main_exe_addr: Address) -> Memory {
        Memory {
            namedata_ptr: DeepPointer::new(main_exe_addr, asr::PointerSize::Bit64, &[0x9F8E10]),
            player_actor_class_ptr: DeepPointer::new(
                main_exe_addr,
                asr::PointerSize::Bit64,
                &[0x7043C0, 0x0, 0x8],
            ),
            all_classes_ptr: DeepPointer::new(main_exe_addr, asr::PointerSize::Bit64, &[0x9F8980]),
        }
    }
}
