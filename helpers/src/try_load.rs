use std::error::Error;
use std::future::Future;
use std::time::Duration;

pub async fn wait_try_load<T, F, Fut>(load_fn: F) -> T
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Box<dyn Error>>>,
{
    let cooldown = Duration::from_millis(100);
    asr::print_message("=> attempting try_load");

    let result = loop {
        let result = load_fn().await;

        if result.is_ok() {
            break result.unwrap();
        }

        // SAFETY: we just checked it's an error above
        //   (this is to avoid the Debug constraint)
        let error = unsafe { result.unwrap_err_unchecked() };

        asr::print_message(&format!(
            "=> try_load unsuccessful, trying again in {}ms! with error: {}",
            cooldown.as_millis(),
            error
        ));
        asr::future::sleep(cooldown).await;
    };

    result
}
