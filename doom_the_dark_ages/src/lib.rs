use asr::signature::Signature;
use asr::{future::next_tick, PointerSize, Process};
use bytemuck::CheckedBitPattern;
use helpers::error::SimpleError;
use helpers::memory::scan_rel;
use helpers::pointer::MemoryWatcher;
use idtech;
use std::error::Error;

extern crate helpers;
use idtech::{IdTech, IdTechVersion};

asr::async_main!(stable);

async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        asr::print_message(&panic_info.to_string());
    }));

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("DOOMTheDarkAges.exe").await;
        process
            .until_closes(async {
                on_attach(&process).await.expect("problem");
            })
            .await;
    }
}

static GAME_SYSTEM_LOCAL_SIG: Signature<17> =
    Signature::new("FF 50 40 48 8D 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 84 C0");

#[derive(CheckedBitPattern, Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
enum IdGameSystemLocalState {
    MainMenu = 0,
    Loading = 1,
    InGame = 2,
}

async fn on_attach(process: &Process) -> Result<(), Box<dyn Error>> {
    let idtech = helpers::try_load::wait_try_load::<IdTech, _, _>(async || {
        IdTech::try_load(process, IdTechVersion::IdTech8, "DOOMTheDarkAges.exe").await
    })
    .await;

    let game_system_local = scan_rel(
        &GAME_SYSTEM_LOCAL_SIG,
        process,
        "DOOMTheDarkAges.exe",
        0x6,
        0x4,
    )?;
    asr::print_message(&format!(
        "=> found idGameSystemLocal ptr at 0x{}",
        game_system_local
    ));
    // let game_system_local = process
    //     .read_pointer(game_system_local_ptr, PointerSize::Bit64)
    //     .map_err(|_| SimpleError::from("unable to read idGameSystemLocal pointer"))?;
    // asr::print_message(&format!(
    //     "=> found idGameSystemLocal instance at 0x{}",
    //     game_system_local
    // ));

    // Get the classes we need - we assume that they exist by now,
    //   if they don't, it's a fatal error and we shouldn't retry
    let mut state = MemoryWatcher::<IdGameSystemLocalState, 1>::new(
        process,
        game_system_local,
        [idtech.get_offset("Game", "idGameSystemLocal", "state")? as u64],
    );

    loop {
        if state.changed()? {
            asr::print_message(&format!(
                "state changed from {:?} to {:?}",
                state.old(),
                state.current()?
            ))
        }

        // Prepare for the next iteration
        state.next_tick();

        next_tick().await;
    }

    Ok(())
}
