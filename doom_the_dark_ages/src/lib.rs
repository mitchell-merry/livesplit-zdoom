use asr::{future::next_tick, Process};
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

async fn on_attach(process: &Process) -> Result<(), Box<dyn Error>> {
    let idtech = helpers::try_load::wait_try_load::<IdTech, _, _>(async || {
        IdTech::try_load(process, IdTechVersion::IdTech8, "DOOMTheDarkAges.exe").await
    })
    .await;

    // Get the classes we need - we assume that they exist by now,
    //   if they don't, it's a fatal error and we shouldn't retry
    let game_system_local = idtech.get_class("Game", "idGameSystemLocal")?;
    let state_offset = game_system_local.get_variable("state")?.get_offset()?;

    loop {
        next_tick().await;
    }

    Ok(())
}
