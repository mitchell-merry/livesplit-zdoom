#![feature(type_alias_impl_trait, const_async_blocks)]

mod zdoom;

use asr::{deep_pointer::DeepPointer, future::next_tick, watcher::Watcher, Address, Process};
use zdoom::ZDoom;

asr::async_main!(nightly);

async fn main() {
    // TODO: Set up some general state and settings.

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("lzdoom.exe").await;
        process
            .until_closes(async {
                // let mut watchers = Watchers::default();
                // watchers.update(&process, &memory);

                let zdoom = ZDoom::load(&process).expect("failed loading zdoom");
                zdoom.show_all_classes();

                // if let Some(inventory) = class_manager.find_class("Inventory") {
                //     inventory.debug_all_fields(&process, &name_data).expect("wah");
                // }

                // class_manager.show_all_classes(&process);

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

// #[derive(Default)]
// struct Watchers {
//     name_manager_base: Watcher<Address>,
//     player_actor_class: Watcher<u64>,
// }

// impl Watchers {
//     fn update(&mut self, game: &Process, memory: &Memory) {
//         self.name_manager_base
//             .update(memory.namedata_ptr.deref_offsets(game).ok());
//         self.player_actor_class
//             .update(memory.player_actor_class_ptr.deref(game).ok());
//     }
// }
