use asr::{Address, Process};
use std::cell::OnceCell;

pub struct ClassTypeInfo<'a> {
    process: &'a Process,
    address: Address,

    name: String,
    size: OnceCell<u32>,
}
