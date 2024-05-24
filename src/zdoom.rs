use std::{marker::PhantomData, mem, ops::Add, process};

use asr::{print_message, string::ArrayCString, Address, Error, Process};
use bytemuck::CheckedBitPattern;

const NAME_ENTRY_SIZE: u64 = 0x10;

const PFIELD_NAME: u64 = 0x28;
const PFIELD_OFFSET: u64 = 0x38;
const PFIELD_TYPE: u64 = 0x40;
const PFIELD_FLAGS: u64 = 0x50;

const PCLASS_TYPENAME: u64 = 0x38;
const PCLASS_DESCRIPTIVE_NAME: u64 = 0x88;

pub struct TArray<T: CheckedBitPattern> {
    _phantom: PhantomData<T>,
    address: Address,
}

impl<T: CheckedBitPattern> TArray<T> {
    pub fn new(address: Address) -> TArray<T> {
        TArray {
            _phantom: PhantomData,
            address,
        }
    }

    pub fn into_iter<'a>(&self, process: &'a Process) -> Result<TArrayIterator<'a, T>, Error> {
        let size =
            process.read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x8 as u64])?;
        Ok(TArrayIterator::<T> {
            _phantom: PhantomData,
            process,
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

        // asr::print_message(&format!("{}/{}", self.index, self.size));

        let offset = (std::mem::size_of::<T>() as u32 * self.index) as u64;
        // let offset = (0x10 * self.index) as u64;
        // asr::print_message(&format!("offset: {offset:X}"));

        // let item = self
        //     .process
        //     .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x0])
        //     .ok()
        //     .unwrap();
        // asr::print_message(&format!("addr: {item:X}"));

        let item = self
            .process
            .read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x0, offset])
            .ok();

        self.index = self.index + 1;

        return item;
    }
}

pub struct NameManager {
    address: Address,
}

impl NameManager {
    pub fn new(address: Address) -> NameManager {
        NameManager { address }
    }

    pub fn get_chars(&self, process: &Process, index: u32) -> Result<String, Error> {
        // asr::print_message(&format!("using thing: {} {index}", self.address.add(0x8)));

        let read = process.read_pointer_path::<ArrayCString<128>>(
            self.address,
            asr::PointerSize::Bit64,
            &[0x8, index as u64 * NAME_ENTRY_SIZE, 0x0],
        );

        // let a = process.read_pointer_path::<u64>(
        //     self.address,
        //     asr::PointerSize::Bit64,
        //     &[0x8, index as u64 * NAME_ENTRY_SIZE],
        // )?;
        // asr::print_message(&format!("check: {:X}", a));

        // let a = process.read

        let read = match read {
            Ok(cstr) => cstr,
            Err(e) => {
                asr::print_message(&format!("what? a{e:?}"));
                panic!("waaaah..");
            }
        };

        match read.validate_utf8() {
            Ok(s) => Ok(s.to_owned()),
            Err(e) => {
                asr::print_message(&format!("what? {e}"));
                panic!("waaaah..");
            }
        }
    }
}

pub struct PClass {
    address: Address,
}

impl PClass {
    pub fn new(addr: Address) -> PClass {
        PClass { address: addr }
    }

    pub fn name(&self, process: &Process, name_data: &NameManager) -> Result<String, Error> {
        // let vm_type = PType {
        //     address: process
        //         .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x88])?
        //         .into(),
        // };

        // return vm_type.name(process);
        return name_data.get_chars(
            process,
            process.read_pointer_path::<u32>(
                self.address,
                asr::PointerSize::Bit64,
                &[PCLASS_TYPENAME],
            )?,
        );
    }

    pub fn raw_name(&self, process: &Process) -> Result<String, Error> {
        let vm_type = PType {
            address: process
                .read_pointer_path::<u64>(
                    self.address,
                    asr::PointerSize::Bit64,
                    &[PCLASS_DESCRIPTIVE_NAME],
                )?
                .into(),
        };

        return vm_type.name(process);
    }

    pub fn debug_all_fields(
        &self,
        process: &Process,
        name_data: &NameManager,
    ) -> Result<(), Error> {
        let parent_class: Address = process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x0])?
            .into();

        if parent_class != Address::NULL {
            let parent_class = PClass::new(parent_class);
            parent_class.debug_all_fields(process, name_data)?;
        }

        asr::print_message(&format!("{} fields:", self.name(process, name_data)?));

        let fields_addr = self.address.add(0x78);
        // process.read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x78])?;
        asr::print_message(&format!("fields_addr {:?}", fields_addr));
        let field_addrs = TArray::<u64>::new(fields_addr.into());
        // asr::print_message("a");

        for field_addr in field_addrs.into_iter(process)? {
            // asr::print_message(&format!("field_addr {:X}", field_addr));
            let field = PField::new(field_addr.into());
            // asr::print_message("a");
            asr::print_message(&format!(
                "  => 0x{:X}  {}: {} [{:X}]",
                field.offset(process)?,
                field.name(process, name_data)?,
                field.ptype(process)?.name(process)?,
                field_addr
            ));
        }

        Ok(())
    }
}

struct PField {
    address: Address,
}

impl PField {
    pub fn new(address: Address) -> Self {
        PField { address }
    }

    pub fn name(&self, process: &Process, name_data: &NameManager) -> Result<String, Error> {
        // asr::print_message(&format!("what {}", self.address));
        let name_index: u32 =
            process.read_pointer_path(self.address, asr::PointerSize::Bit64, &[PFIELD_NAME])?;
        // asr::print_message("a");

        name_data.get_chars(process, name_index)
    }

    pub fn offset(&self, process: &Process) -> Result<u32, Error> {
        process.read_pointer_path(self.address, asr::PointerSize::Bit64, &[PFIELD_OFFSET])
    }

    pub fn ptype(&self, process: &Process) -> Result<PType, Error> {
        let ptype: Address = process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[PFIELD_TYPE])?
            .into();
        Ok(PType::new(ptype))
    }

    // pub fn flags*&self, process: &Process) -> Result<
}

struct PType {
    address: Address,
}

impl PType {
    fn new(address: Address) -> PType {
        PType { address }
    }

    fn name(&self, process: &Process) -> Result<String, Error> {
        let c_str = process.read_pointer_path::<ArrayCString<128>>(
            self.address,
            asr::PointerSize::Bit64,
            &[0x48, 0x0],
        );

        let b = c_str?
            .validate_utf8()
            .expect("name should always be utf-8")
            .to_owned();

        return Ok(b);
    }
}
