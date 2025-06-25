use crate::error::SimpleError;
use asr::{Address, PointerSize, Process};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;
use std::error::Error;

pub struct MemoryWatcher<'a, T: CheckedBitPattern, const N: usize> {
    process: &'a Process,
    base_address: Address,
    path: [u64; N],

    current: OnceCell<T>,
    old: Option<T>,
}

impl<'a, T: CheckedBitPattern + PartialEq + std::fmt::Debug, const N: usize>
    MemoryWatcher<'a, T, N>
{
    pub fn new(process: &'a Process, base_address: Address, path: [u64; N]) -> Self {
        MemoryWatcher {
            process,
            base_address,
            path,

            current: OnceCell::new(),
            old: None,
        }
    }

    pub fn current(&self) -> Result<&T, Box<dyn Error>> {
        self.current.get_or_try_init(|| {
            let val = self
                .process
                .read_pointer_path::<T>(self.base_address, PointerSize::Bit64, &self.path)
                .map_err(|_| SimpleError::from("unable to read value from pointer path"))?;

            Ok(val)
        })
    }

    pub fn old(&self) -> &Option<T> {
        &self.old
    }

    pub fn changed(&self) -> Result<bool, Box<dyn Error>> {
        match self.old {
            None => Ok(true),
            Some(old) => Ok(&old != self.current()?),
        }
    }

    pub fn next_tick(&mut self) {
        self.old = self.current.get().copied();
        self.current = OnceCell::new();
    }
}
