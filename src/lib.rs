#![feature(type_alias_impl_trait, const_async_blocks)]

mod zdoom;

use std::error::Error;

use asr::{deep_pointer::DeepPointer, future::next_tick, watcher::Watcher, Address, Process};
use zdoom::{NameManager, PClass, PClassManager, TArray};

asr::async_main!(nightly);

async fn main() {
    // TODO: Set up some general state and settings.

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("lzdoom.exe").await;
        process
            .until_closes(async {
                asr::print_message("a");
                let main_exe_addr = process
                    .get_module_address("lzdoom.exe")
                    .expect("failed getting lzdoom");
                asr::print_message("a");
                let memory = Memory::new(main_exe_addr);
                let mut watchers = Watchers::default();
                watchers.update(&process, &memory);

                // asr::print_message("a");
                let name_data = NameManager::new(
                    watchers
                        .name_manager_base
                        .pair
                        .expect("need a valid thingy")
                        .current,
                );

                let all_classes_addr = memory
                .all_classes_ptr
                .deref_offsets(&process)
                .expect("wah?");

                let class_manager = PClassManager::load(&process, &name_data, all_classes_addr).expect("wah");

                if let Some(inventory) = class_manager.find_class("Inventory") {
                    inventory.debug_all_fields(&process, &name_data).expect("wah");
                }

                // asr::print_message("a");
                // if let Some(pactor_class) = &watchers.player_actor_class.pair {
                //     let class = PClass::new(pactor_class.current.into());
                //     asr::print_message("a");
                //     class.debug_all_fields(&process, &name_data).expect("aaah");
                //     asr::print_message("a");

                //     let name = class.name(&process, &name_data);
                //     match name {
                //         Ok(name) => {
                //             asr::timer::set_variable("class", &name);
                //         }
                //         Err(error) => {
                //             asr::timer::set_variable("class", &format!("{:?}", error));
                //         }
                //     }
                // } else {
                //     asr::timer::set_variable("class", "unknown");
                // }

                // TODO: Load some initial information from the process.
                loop {
                    next_tick().await;
                }
            })
            .await;
    }
}

#[derive(Default)]
struct Watchers {
    name_manager_base: Watcher<Address>,
    player_actor_class: Watcher<u64>,
}

impl Watchers {
    fn update(&mut self, game: &Process, memory: &Memory) {
        self.name_manager_base
            .update(memory.namedata_ptr.deref_offsets(game).ok());
        self.player_actor_class
            .update(memory.player_actor_class_ptr.deref(game).ok());
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
