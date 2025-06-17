use crate::typeinfo::{
    TYPE_INFO_PROJECT_NAME_OFFSET, TYPE_INFO_PROJECT_TYPE_INFO_GENERATED_OFFSET,
};
use asr::string::ArrayCString;
use asr::{Address, Process};
use helpers::error::SimpleError;
use std::cell::OnceCell;
use std::error::Error;

const CLASS_TYPE_INFO_NAME_OFFSET: u64 = 0x0;

pub struct ClassTypeInfo<'a> {
    process: &'a Process,
    address: Address,

    pub name: String,
    size: OnceCell<u32>,
}

impl<'a> ClassTypeInfo<'a> {
    pub fn init(
        process: &'a Process,
        address: Address,
    ) -> Result<ClassTypeInfo<'a>, Box<dyn Error>> {
        let name = process
            .read_pointer_path::<ArrayCString<512>>(
                address,
                asr::PointerSize::Bit64,
                &[CLASS_TYPE_INFO_NAME_OFFSET, 0x0],
            )
            .map_err(|_| SimpleError::from("failed to read name of class"))?
            .validate_utf8()?
            .to_owned();

        Ok(ClassTypeInfo {
            process,
            address,
            name,

            size: OnceCell::new(),
        })
    }
}
