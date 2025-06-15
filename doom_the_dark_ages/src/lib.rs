use asr::{future::next_tick, timer, watcher::Watcher, Error, Process};
use idtech;

#[macro_use]
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

async fn on_attach(process: &Process) -> Result<(), Option<Error>> {
    let (mut idtech, _) = IdTech::wait_try_load(
        process,
        IdTechVersion::IdTech8,
        "DOOMTheDarkAges.exe",
        |_| Ok(()),
    )
    .await;

    loop {
        next_tick().await;
    }

    Ok(())
}
