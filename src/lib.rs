#![feature(type_alias_impl_trait, const_async_blocks)]

mod zdoom;

use asr::{deep_pointer::DeepPointer, future::next_tick, watcher::Watcher, Address, Process};
use zdoom::{ZDoom, ZDoomVersion};

asr::async_main!(nightly);

async fn main() {
    // TODO: Set up some general state and settings.
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("gzdoom.exe").await;
        process
            .until_closes(async {
                // let mut watchers = Watchers::default();
                // watchers.update(&process, &memory);

                let mut zdoom =
                    ZDoom::load(&process, ZDoomVersion::Gzdoom4_8_2).expect("failed loading zdoom");
                // let mut zdoom =
                //     ZDoom::load(&process, ZDoomVersion::Lzdoom3_82).expect("failed loading zdoom");

                // zdoom.show_all_classes();
                // if let Some(class) = zdoom.find_class("SnapPlayer") {
                //     class.debug_all_fields(&zdoom.name_data).expect("bwa");
                // }

                loop {
                    let name = zdoom.level.name();
                    if let Ok(name) = name {
                        asr::timer::set_variable("map", name);
                    } else {
                        asr::timer::set_variable("map", "failed reading map!");
                    }

                    let pos = zdoom.player.pos();
                    if let Ok(pos) = pos {
                        asr::timer::set_variable("pos", &format!("{pos:?}"));
                    } else {
                        asr::timer::set_variable("pos", "failed reading pos!");
                    }

                    zdoom.invalidate_cache().expect("ah");
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
