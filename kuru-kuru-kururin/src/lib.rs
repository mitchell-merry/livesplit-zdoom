#[macro_use]
extern crate helpers;
use asr::emulator::gba::Emulator;
use asr::future::next_tick;
use asr::settings::Gui;
use asr::time::Duration;
use asr::timer::{set_game_time, set_variable, start};
use asr::Error;
use bitflags::{bitflags, Flags};
use bytemuck::{CheckedBitPattern, Pod, Zeroable};
use helpers::pointer::{Invalidatable, MemoryWatcher, PointerPath};
use std::fmt::Debug;

asr::async_main!(stable);

// pub struct GbaEmulatorReadable<'a> {
//     emulator: &'a Emulator,
// }
//
// impl<'a> From<&'a Emulator> for GbaEmulatorReadable<'a> {
//     fn from(value: &'a Emulator) -> Self {
//         GbaEmulatorReadable { emulator: value }
//     }
// }

#[derive(Gui)]
struct Settings {}
async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Attempting to attach...");
    // let mut buf = [MaybeUninit::uninit()];
    // let x = Process::list_by_name_into("", &mut buf);
    // let out = buf.get(0).unwrap();
    // let x = unsafe { out.assume_init() };
    // asr::print_message(&format!("{:?}", x));

    let mut settings = Settings::register();

    loop {
        let emulator = Emulator::wait_attach().await;
        emulator
            .until_closes(async {
                on_attach(&emulator, &mut settings).await.expect("problem");
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

        // The following are unused, just some I happened to figure out

        /// the player has completed the level (reset to 0 when a level starts)
        const HasFinished = 1 << 3;
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

async fn on_attach(emulator: &Emulator, _settings: &mut Settings) -> Result<(), Option<Error>> {
    asr::print_message("Attached!");
    set_variable(
        "ram base (ewram)",
        &format!("0x{}", emulator.ram_base.get().unwrap().get(0).unwrap()),
    );
    set_variable(
        "ram base (iwram)",
        &format!("0x{}", emulator.ram_base.get().unwrap().get(1).unwrap()),
    );
    let some_important_thing = PointerPath::new32(emulator, 0x03004420_u64.into(), &[0x18, 0x0]);

    let mut time_pointer: MemoryWatcher<_, u32> = some_important_thing.child(&[0xB8]).into();
    let mut flags_pointer: MemoryWatcher<_, GameFlags> = some_important_thing.child(&[0xBC]).into();

    while emulator.is_open() {
        let time = time_pointer.current_owned().unwrap_or(0);
        set_variable("time (frames)", &format!("{time}"));

        let old_flags = flags_pointer.old_owned().unwrap_or_default();
        let flags = flags_pointer.current_owned().unwrap_or_default();
        set_variable("flags", &format!("{:08x}", flags.bits()));
        set_variable("flags (interpreted)", &format!("{:?}", flags));

        // Ok
        if !old_flags.contains(GameFlags::HasStarted) && flags.contains(GameFlags::HasStarted) {
            start();
        }

        set_game_time(get_in_game_time(time));

        time_pointer.next_tick();
        flags_pointer.next_tick();
        next_tick().await;
    }
    Ok(())
}
