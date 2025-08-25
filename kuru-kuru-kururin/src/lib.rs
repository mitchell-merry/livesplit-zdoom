#[macro_use]
extern crate helpers;

use asr::emulator::gba::Emulator;
use asr::future::next_tick;
use asr::settings::Gui;
use asr::time::Duration;
use asr::timer::{
    pause_game_time, resume_game_time, set_game_time, set_variable, start, state, TimerState,
};
use bitflags::{bitflags, Flags};
use bytemuck::{CheckedBitPattern, Pod, Zeroable};
use helpers::pointer::{Invalidatable, MemoryWatcher, PointerPath};
use helpers::settings::initialise_settings;
use helpers::{better_split, get_setting};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Debug;

asr::async_main!(stable);

#[derive(Gui)]
struct Settings {}

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Attempting to attach...");

    let settings_defaults = initialise_settings(include_str!("../data/settings.ron"))
        .expect("failed to initialise settings");

    loop {
        let emulator = Emulator::wait_attach().await;
        emulator
            .until_closes(async {
                on_attach(&emulator, &settings_defaults)
                    .await
                    .expect("problem");
            })
            .await;
    }
}

// 3004430

// 0x30045EC important flags

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    #[repr(C)]
    pub struct GameFlags: u32 {
        /// the timer is running (reset to 0 when a level starts)
        const HasStarted = 1 << 1;
        /// the player has completed the level (reset to 0 when a level starts)
        const HasFinished = 1 << 3;

        // The following are unused, just some I happened to figure out

        /// the player has died
        const PlayerIsDead = 1 << 5;

        /// player is holding down 1 level of sprint (but not 2)
        const PlayerIsSprinting = 1 << 12;
        /// player is holding down 2 levels of sprint
        const PlayerIsReallySprinting = 1 << 13;
        /// player is in "start" or "checkpoints" squares and should be healed
        const PlayerInHealer = 1 << 18;

        // Make all other bits "known" (for the purpose of displaying them in debug output)
        const _ = !0;
    }
}

fn get_in_game_time(frames: u32) -> Duration {
    Duration::seconds_f32((frames as f32) / 60_f32)
}

fn flag_just_enabled(
    flags_watcher: &MemoryWatcher<Emulator, GameFlags>,
    flag: GameFlags,
) -> Result<bool, Box<dyn Error>> {
    let old = match flags_watcher.old_owned() {
        Some(x) => x,
        None => return Ok(false),
    };

    let current = match flags_watcher.current_owned() {
        Ok(x) => x,
        Err(e) => return Err(e),
    };

    Ok(!old.contains(flag) && current.contains(flag))
}

async fn on_attach(
    emulator: &Emulator,
    settings_defaults: &HashMap<String, bool>,
) -> Result<(), Box<dyn Error>> {
    asr::print_message("Attached!");

    set_variable(
        "ram base (ewram)",
        &format!("0x{}", emulator.ram_base.get().unwrap().get(0).unwrap()),
    );
    set_variable(
        "ram base (iwram)",
        &format!("0x{}", emulator.ram_base.get().unwrap().get(1).unwrap()),
    );
    let base = PointerPath::new32(emulator, 0x3004420_u64.into(), &[]);
    let mut world: MemoryWatcher<_, u8> = base.child(&[0x0]).into();
    let mut sub_level: MemoryWatcher<_, u8> = base.child(&[0x1]).into();

    let some_important_thing = base.child(&[0x18, 0x0]);

    let mut time: MemoryWatcher<_, u32> = some_important_thing.child(&[0xB8]).into();
    let mut flags_pointer: MemoryWatcher<_, GameFlags> = some_important_thing.child(&[0xBC]).into();

    let mut completed_splits = HashSet::new();

    while emulator.is_open() {
        set_variable("time (frames)", &format!("{}", time.current_owned()?));

        let flags = flags_pointer.current_owned()?;
        set_variable("flags", &format!("{:08x}", flags.bits()));
        set_variable("flags (interpreted)", &format!("{:?}", flags));

        set_variable("world", &format!("{}", world.current_owned()?));
        set_variable("sub level", &format!("{:?}", sub_level.current_owned()?));

        // IL start
        if get_setting("il_mode", &settings_defaults)?
            && flag_just_enabled(&flags_pointer, GameFlags::HasStarted)?
        {
            start();
        }

        if state() == TimerState::Running {
            if get_setting("igt_mode", &settings_defaults)? {
                set_game_time(get_in_game_time(time.current_owned()?));
                pause_game_time();
            } else {
                resume_game_time();
            }
        }

        // Splits (level completion)
        if flag_just_enabled(&flags_pointer, GameFlags::HasFinished)? {
            let key = &format!(
                "_level_{:?}_{}",
                world.current_owned()?,
                sub_level.current_owned()?
            );
            let _ = better_split(key, &settings_defaults, &mut completed_splits);
        }

        world.next_tick();
        sub_level.next_tick();
        time.next_tick();
        flags_pointer.next_tick();
        next_tick().await;
    }
    Ok(())
}
