use asr::Process;
use std::error::Error;

pub fn try_load<T, F>(load_fn: F) -> Result<T, Option<Box<dyn Error>>>
where
    F: Fn() -> Result<T, Option<Box<dyn Error>>>,
{
    load_fn()
}
