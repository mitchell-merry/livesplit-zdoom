use std::marker::PhantomData;

use asr::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

pub struct TArray<'a, T: CheckedBitPattern> {
    _phantom: PhantomData<T>,
    process: &'a Process,
    address: Address,
}

impl<'a, T: CheckedBitPattern> TArray<'a, T> {
    pub fn new(process: &'a Process, address: Address) -> TArray<'a, T> {
        TArray {
            _phantom: PhantomData,
            process,
            address,
        }
    }

    pub fn into_iter(&self) -> Result<TArrayIterator<'a, T>, Error> {
        let size =
            self.process
                .read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x8_u64])?;
        Ok(TArrayIterator::<T> {
            _phantom: PhantomData,
            process: self.process,
            address: self.address,
            size,
            index: 0,
        })
    }
}

pub struct TArrayIterator<'a, T: CheckedBitPattern> {
    _phantom: PhantomData<T>,
    process: &'a Process,
    address: Address,
    size: u32,
    index: u32,
}

impl<'a, T: CheckedBitPattern> Iterator for TArrayIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.size {
            return None;
        }

        let offset = std::mem::size_of::<T>() * self.index as usize;
        let item = self
            .process
            .read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x0, offset as u64])
            .ok();

        self.index += 1;

        item
    }
}
