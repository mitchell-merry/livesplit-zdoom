use std::{collections::HashMap, fmt::Display, marker::PhantomData, mem, ops::Add, process};

use asr::{print_message, string::ArrayCString, Address, Error, Process};
use bitflags::bitflags;
use bytemuck::CheckedBitPattern;

const NAME_ENTRY_SIZE: u64 = 0x10;

const PFIELD_NAME: u64 = 0x28;
const PFIELD_OFFSET: u64 = 0x38;
const PFIELD_TYPE: u64 = 0x40;
const PFIELD_FLAGS: u64 = 0x48;

const PCLASS_TYPENAME: u64 = 0x38;
const PCLASS_DESCRIPTIVE_NAME: u64 = 0x88;

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
                .read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x8 as u64])?;
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

        let offset = std::mem::size_of::<T>() as u64 * self.index as u64;
        let item = self
            .process
            .read_pointer_path(self.address, asr::PointerSize::Bit64, &[0x0, offset])
            .ok();

        self.index = self.index + 1;

        return item;
    }
}

pub struct NameManager<'a> {
    process: &'a Process,
    address: Address,
}

impl<'a> NameManager<'a> {
    pub fn new(process: &'a Process, address: Address) -> NameManager<'a> {
        NameManager { process, address }
    }

    pub fn get_chars(&self, index: u32) -> Result<String, Error> {
        let read = self.process.read_pointer_path::<ArrayCString<128>>(
            self.address,
            asr::PointerSize::Bit64,
            &[0x8, index as u64 * NAME_ENTRY_SIZE, 0x0],
        );

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

pub struct PClass<'a> {
    process: &'a Process,
    name_data: &'a NameManager<'a>,
    address: Address,
}

impl<'a> PClass<'a> {
    pub fn new(process: &'a Process, name_data: &'a NameManager, addr: Address) -> PClass<'a> {
        PClass {
            process,
            name_data,
            address: addr,
        }
    }

    pub fn name(&self) -> Result<String, Error> {
        return self
            .name_data
            .get_chars(self.process.read_pointer_path::<u32>(
                self.address,
                asr::PointerSize::Bit64,
                &[PCLASS_TYPENAME],
            )?);
    }

    pub fn raw_name(&self, process: &Process) -> Result<String, Error> {
        let vm_type = PType::new(
            process,
            process
                .read_pointer_path::<u64>(
                    self.address,
                    asr::PointerSize::Bit64,
                    &[PCLASS_DESCRIPTIVE_NAME],
                )?
                .into(),
        );

        return vm_type.name();
    }

    pub fn debug_all_fields(&self) -> Result<(), Error> {
        let parent_class: Address = self
            .process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x0])?
            .into();

        if parent_class != Address::NULL {
            let parent_class = PClass::new(self.process, self.name_data, parent_class);
            parent_class.debug_all_fields()?;
        }

        asr::print_message(&format!("{} fields:", self.name()?));

        let fields_addr = self.address.add(0x78);
        let field_addrs = TArray::<u64>::new(self.process, fields_addr.into());

        for field_addr in field_addrs.into_iter()? {
            let field = PField::new(&self.process, &self.name_data, field_addr.into());
            let flags = field.flags()?;
            let is_static = match flags.contains(PFieldFlags::VARF_Static) {
                true => " static",
                false => "       ",
            };

            asr::print_message(&format!(
                "  => {is_static} 0x{:X}  {}: {} [{:?}]",
                field.offset()?,
                field.name()?,
                field.ptype()?.name()?,
                flags
            ));
        }

        Ok(())
    }
}

pub struct PClassManager<'a> {
    process: &'a Process,
    name_data: &'a NameManager<'a>,
    classes: HashMap<String, PClass<'a>>,
}

impl<'a> PClassManager<'a> {
    pub fn load(
        process: &'a Process,
        name_data: &'a NameManager,
        address: Address,
    ) -> Result<Self, Error> {
        let mut classes: HashMap<String, PClass> = HashMap::new();

        let all_classes = TArray::<u64>::new(process, address);

        for class in all_classes.into_iter()? {
            let pclass = PClass::new(process, name_data, class.into());
            let name = pclass.name()?;

            classes.insert(name, pclass);
        }

        Ok(PClassManager {
            process,
            name_data,
            classes,
        })
    }

    pub fn find_class(&self, name: &str) -> Option<&PClass> {
        self.classes.get(name)
    }

    pub fn show_all_classes(&self) {
        for (name, _class) in self.classes.iter() {
            asr::print_message(name);
        }
    }

    pub fn dump(&self) {}
}

bitflags! {
    #[derive(Debug, PartialEq)]
    struct PFieldFlags: u32 {
        const VARF_Optional = 1 << 0;        // func param is optional
        const VARF_Method = 1 << 1;          // func has an implied self parameter
        const VARF_Action = 1 << 2;          // func has implied owner and state parameters
        const VARF_Native = 1 << 3;	         // func is native code, field is natively defined
        const VARF_ReadOnly = 1 << 4;        // field is read only, do not write to it
        const VARF_Private = 1 << 5;         // field is private to containing class
        const VARF_Protected = 1 << 6;       // field is only accessible by containing class and children.
        const VARF_Deprecated = 1 << 7;	     // Deprecated fields should output warnings when used.
        const VARF_Virtual = 1 << 8;	     // function is virtual
        const VARF_Final = 1 << 9;	         // Function may not be overridden in subclasses
        const VARF_In = 1 << 10;
        const VARF_Out = 1 << 11;
        const VARF_Implicit = 1 << 12;	     // implicitly created parameters (i.e. do not compare types when checking function signatures)
        const VARF_Static = 1 << 13;
        const VARF_InternalAccess = 1 << 14; // overrides VARF_ReadOnly for internal script code.
        const VARF_Override = 1 << 15;	     // overrides a virtual function from the parent class.
        const VARF_Ref = 1 << 16;	         // argument is passed by reference.
        const VARF_Transient = 1 << 17;      // don't auto serialize field.
        const VARF_Meta = 1 << 18;	         // static class data (by necessity read only.)
        const VARF_VarArg = 1 << 19;         // [ZZ] vararg: don't typecheck values after ... in function signature
        const VARF_UI = 1 << 20;             // [ZZ] ui: object is ui-scope only (can't modify playsim)
        const VARF_Play = 1 << 21;           // [ZZ] play: object is playsim-scope only (can't access ui)
        const VARF_VirtualScope	= 1 << 22;   // [ZZ] virtualscope: object should use the scope of the particular class it's being used with (methods only)
        const VARF_ClearScope = 1 << 23;     // [ZZ] clearscope: this method ignores the member access chain that leads to it and is always plain data.
    }
}

struct PField<'a> {
    process: &'a Process,
    name_data: &'a NameManager<'a>,
    address: Address,
}

impl<'a> PField<'a> {
    pub fn new(process: &'a Process, name_data: &'a NameManager, address: Address) -> PField<'a> {
        PField {
            process,
            name_data,
            address,
        }
    }

    pub fn name(&self) -> Result<String, Error> {
        let name_index: u32 = self.process.read_pointer_path(
            self.address,
            asr::PointerSize::Bit64,
            &[PFIELD_NAME],
        )?;

        self.name_data.get_chars(name_index)
    }

    pub fn offset(&self) -> Result<u32, Error> {
        self.process
            .read_pointer_path(self.address, asr::PointerSize::Bit64, &[PFIELD_OFFSET])
    }

    pub fn ptype(&self) -> Result<PType, Error> {
        let ptype: Address = self
            .process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[PFIELD_TYPE])?
            .into();
        Ok(PType::new(self.process, ptype))
    }

    pub fn flags(&self) -> Result<PFieldFlags, Error> {
        Ok(PFieldFlags::from_bits_truncate(
            self.process.read_pointer_path::<u32>(
                self.address,
                asr::PointerSize::Bit64,
                &[PFIELD_FLAGS],
            )?,
        ))
    }
}

struct PType<'a> {
    process: &'a Process,
    address: Address,
}

impl<'a> PType<'a> {
    fn new(process: &'a Process, address: Address) -> PType<'a> {
        PType { process, address }
    }

    fn name(&self) -> Result<String, Error> {
        let c_str = self.process.read_pointer_path::<ArrayCString<128>>(
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
