#[macro_use]
extern crate helpers;
use asr::emulator::gba::Emulator;
use asr::future::next_tick;
use asr::settings::Gui;
use asr::time::Duration;
use asr::timer::{set_game_time, set_variable};
use asr::Error;
use bitflags::bitflags;
use std::fmt::Debug;

asr::async_main!(stable);

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
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct GameFlags: u32 {
        const HasStarted = 1 << 1;
        const HasFinished = 1 << 3;
        const InHealer = 1 << 18;
    }
}

fn get_in_game_time(frames: u32) -> Duration {
    Duration::seconds_f32((frames as f32) / 60_f32)
}

async fn on_attach(emulator: &Emulator, _settings: &mut Settings) -> Result<(), Option<Error>> {
    asr::print_message("Attached!");
    // asr::print_message(&format!("{:?}", emulator.ram_base.get()));
    while emulator.is_open() {
        let some_important_offset = 0x03004420;
        let a = emulator.read::<u32>(some_important_offset + 0x18)?;

        let time = emulator.read::<u32>(a + 0xB8)?;
        set_variable("time (frames)", &format!("{time}"));

        let flags = emulator.read(a + 0xBC)?;
        set_variable("flags", &format!("{flags:08x}"));

        let flags = GameFlags::from_bits_truncate(flags);
        set_variable("flags (interpreted)", &format!("{flags:?}"));

        // Ok

        set_game_time(get_in_game_time(time));

        next_tick().await;
    }
    Ok(())
}
