use crate::error::SimpleError;
use asr::{Address, PointerSize, Process};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;
use std::error::Error;
use std::fmt::Debug;
use std::iter::once;

pub struct PointerPath<'a> {
    process: &'a Process,
    base_address: Address,
    path: Vec<u64>,
}

impl<'a> PointerPath<'a> {
    pub fn new(process: &'a Process, base_address: Address, path: impl Into<Vec<u64>>) -> Self {
        PointerPath {
            process,
            base_address,
            path: path.into(),
        }
    }

    // the first offset of the child path should not dereference
    pub fn child(&self, path: impl Into<Vec<u64>>) -> Self {
        // im so dumb dude i dontcare shut up
        let (original_last, original_prefix) = self.path.split_last().unwrap_or_else(|| (&0, &[]));
        let path = path.into();
        let (child_prefix, rest) = path.split_first().expect("child path is empty");
        let child_prefix = *child_prefix;
        let original_last = *original_last;
        let new_middle_offset = original_last + child_prefix;

        PointerPath::new(
            self.process,
            self.base_address,
            original_prefix
                .to_owned()
                .into_iter()
                .chain(once(new_middle_offset))
                .chain(rest.to_owned())
                .collect::<Vec<_>>(),
        )
    }

    pub fn read<T: CheckedBitPattern>(&self) -> Result<T, Box<dyn Error>> {
        Ok(self
            .process
            .read_pointer_path::<T>(self.base_address, PointerSize::Bit64, &self.path)
            .map_err(|_| SimpleError::from("unable to read value from pointer path"))?)
    }
}

pub trait Invalidatable {
    fn next_tick(&mut self);
}

pub struct MemoryWatcher<'a, T: CheckedBitPattern> {
    path: PointerPath<'a>,
    current: OnceCell<T>,
    old: Option<T>,
}

impl<'a, T: CheckedBitPattern + PartialEq + Debug> MemoryWatcher<'a, T> {
    pub fn new(process: &'a Process, base_address: Address, path: impl Into<Vec<u64>>) -> Self {
        PointerPath::new(process, base_address, path).into()
    }

    pub fn current(&self) -> Result<&T, Box<dyn Error>> {
        self.current.get_or_try_init(|| self.path.read())
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
}

impl<'a, T: CheckedBitPattern> Invalidatable for MemoryWatcher<'a, T> {
    fn next_tick(&mut self) {
        self.old = self.current.get().copied();
        self.current = OnceCell::new();
    }
}

impl<'a, T: CheckedBitPattern> From<PointerPath<'a>> for MemoryWatcher<'a, T> {
    fn from(value: PointerPath<'a>) -> Self {
        MemoryWatcher {
            path: value,
            current: OnceCell::new(),
            old: None,
        }
    }
}
