use crate::error::SimpleError;
#[cfg(feature = "gba")]
use asr::emulator::gba::Emulator;
use asr::{Address, PointerSize, Process};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;
use std::error::Error;
use std::fmt::Debug;
use std::iter::once;

pub trait Readable {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>>;
}

impl<'a> Readable for Process {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>> {
        self.read_pointer_path::<T>(address, pointer_size, &path)
            .map_err(|_| SimpleError::from("unable to read value from pointer path").into())
    }
}

#[cfg(feature = "gba")]
impl<'a> Readable for Emulator {
    fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        _pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Box<dyn Error>> {
        let path = path.iter().map(|o| *o as u32).collect::<Vec<u32>>();
        let path = path.as_slice();
        self.read_pointer_path::<T>(address.into().value() as u32, path)
            .map_err(|_| SimpleError::from("unable to read value from pointer path").into())
    }
}

pub struct PointerPath<'a, R: Readable + ?Sized> {
    readable: &'a R,
    base_address: Address,
    path: Vec<u64>,
    pointer_size: PointerSize,
}

impl<'a, R: Readable + ?Sized> PointerPath<'a, R> {
    pub fn new(readable: &'a R, base_address: Address, path: impl Into<Vec<u64>>) -> Self {
        PointerPath {
            readable,
            pointer_size: PointerSize::Bit64,
            base_address,
            path: path.into(),
        }
    }

    pub fn new32(readable: &'a R, base_address: Address, path: impl Into<Vec<u64>>) -> Self {
        PointerPath {
            readable,
            pointer_size: PointerSize::Bit32,
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

        PointerPath {
            readable: self.readable,
            pointer_size: self.pointer_size,
            base_address: self.base_address,
            path: original_prefix
                .to_owned()
                .into_iter()
                .chain(once(new_middle_offset))
                .chain(rest.to_owned())
                .collect::<Vec<_>>(),
        }
    }

    pub fn child_watcher<T: CheckedBitPattern>(
        &self,
        path: impl Into<Vec<u64>>,
    ) -> MemoryWatcher<'a, R, T> {
        self.child(path).into()
    }

    pub fn read<T: CheckedBitPattern>(&self) -> Result<T, Box<dyn Error>> {
        self.readable
            .read_pointer_path(self.base_address, self.pointer_size, &self.path)
    }
}

pub trait Invalidatable {
    fn next_tick(&mut self);
}

pub struct MemoryWatcher<'a, R: Readable + ?Sized, T: CheckedBitPattern> {
    path: PointerPath<'a, R>,
    current: OnceCell<T>,
    old: Option<T>,
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern + PartialEq + Debug> MemoryWatcher<'a, R, T> {
    pub fn new(readable: &'a R, base_address: Address, path: impl Into<Vec<u64>>) -> Self {
        PointerPath::new(readable, base_address, path).into()
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

    pub fn child(&self, path: impl Into<Vec<u64>>) -> Self {
        self.path.child(path).into()
    }
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern + PartialEq + Debug + Clone>
    MemoryWatcher<'a, R, T>
{
    pub fn current_owned(&self) -> Result<T, Box<dyn Error>> {
        Ok(self.current()?.to_owned())
    }

    pub fn old_owned(&self) -> Option<T> {
        self.old.clone()
    }
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern> Invalidatable for MemoryWatcher<'a, R, T> {
    fn next_tick(&mut self) {
        self.old = self.current.get().copied();
        self.current = OnceCell::new();
    }
}

impl<'a, R: Readable + ?Sized, T: CheckedBitPattern> From<PointerPath<'a, R>>
    for MemoryWatcher<'a, R, T>
{
    fn from(value: PointerPath<'a, R>) -> Self {
        MemoryWatcher {
            path: value,
            current: OnceCell::new(),
            old: None,
        }
    }
}
