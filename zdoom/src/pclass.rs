use std::{cmp::Ordering, collections::HashMap, iter::Once, rc::Rc};

use asr::{string::ArrayCString, Address, Error, Process};
use bitflags::bitflags;
use once_cell::unsync::OnceCell;

use super::{name_manager::NameManager, tarray::TArray, Memory};

const PFIELD_NAME: u64 = 0x28;
const PFIELD_OFFSET: u64 = 0x38;
const PFIELD_TYPE: u64 = 0x40;
const PFIELD_FLAGS: u64 = 0x48;

const PCLASS_TYPENAME: u64 = 0x38;
const PCLASS_PTYPE: u64 = 0x90;

const PTYPE_SIZE: u64 = 0xC;
const PTYPE_ALIGN: u64 = 0x10;
const PTYPE_FLAGS: u64 = 0x14;
const PTYPE_DESCRIPTIVE_NAME: u64 = 0x48;

#[derive(Clone)]
pub struct PClass<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    name_manager: Rc<NameManager<'a>>,
    address: Address,

    name: OnceCell<String>,
    ptype: OnceCell<PType<'a>>,
    fields: OnceCell<HashMap<String, PField<'a>>>, // does not contain fields of superclasses
}

impl<'a> PClass<'a> {
    pub fn new(
        process: &'a Process,
        memory: Rc<Memory>,
        name_manager: Rc<NameManager<'a>>,
        address: Address,
    ) -> PClass<'a> {
        PClass {
            process,
            name_manager,
            memory,
            address,

            name: OnceCell::new(),
            ptype: OnceCell::new(),
            fields: OnceCell::new(),
        }
    }

    pub fn name(&self) -> Result<&String, Error> {
        self.name.get_or_try_init(|| {
            self.name_manager
                .get_chars(self.process.read_pointer_path::<u32>(
                    self.address,
                    asr::PointerSize::Bit64,
                    &[PCLASS_TYPENAME],
                )?)
        })
    }

    pub fn ptype(&self) -> Result<&PType<'a>, Error> {
        self.ptype.get_or_try_init(|| {
            let address = self.process.read::<u64>(self.address + PCLASS_PTYPE)?;

            Ok(PType::new(self.process, address.into()))
        })
    }

    pub fn fields<'b>(&'b self) -> Result<&'b HashMap<String, PField<'a>>, Error> {
        self.fields.get_or_try_init(|| {
            let mut fields = HashMap::new();

            let fields_addr = self.address.add(self.memory.offsets.pclass_fields);
            let field_addrs = TArray::<u64>::new(self.process, fields_addr);

            for field_addr in field_addrs.into_iter()? {
                let field =
                    PField::new(&self.process, self.name_manager.clone(), field_addr.into());
                fields.insert(field.name().map(|f| f.to_owned())?, field);
            }

            Ok(fields)
        })
    }

    pub fn show_class(&self) -> Result<String, Error> {
        let mut struct_out = String::new();

        let parent_class: Address = self
            .process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x0])?
            .into();

        struct_out.push_str(&format!("class {} ", self.name()?));

        if parent_class != Address::NULL {
            let parent_class = PClass::new(
                self.process,
                self.memory.clone(),
                self.name_manager.clone(),
                parent_class,
            );
            struct_out.push_str(&format!(": public {} ", parent_class.name()?));
        }

        struct_out.push_str("{\n");

        let fields = self.fields()?;
        let mut sorted_fields = Vec::<PField<'a>>::new();

        for (_name, field) in fields {
            sorted_fields.push(field.to_owned());
        }

        sorted_fields.sort();

        for field in sorted_fields {
            let modifier = if field.flags()?.contains(PFieldFlags::Static) {
                "static "
            } else {
                ""
            };
            struct_out.push_str(&format!(
                "  {}{} {}; // offset: 0x{:X}, size: 0x{:X}, align: 0x{:X}\n",
                modifier,
                field.ptype()?.name()?,
                field.name()?,
                field.offset()?,
                field.ptype()?.size()?,
                field.ptype()?.align()?,
            ))
        }

        struct_out.push('}');

        Ok(struct_out)
    }

    pub fn debug_all_fields(&self) -> Result<(), Error> {
        let parent_class: Address = self
            .process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x0])?
            .into();

        if parent_class != Address::NULL {
            let parent_class = PClass::new(
                self.process,
                self.memory.clone(),
                self.name_manager.clone(),
                parent_class,
            );
            parent_class.debug_all_fields()?;
        }

        let fields_addr = self.address.add(self.memory.offsets.pclass_fields);

        let field_addrs = TArray::<u64>::new(self.process, fields_addr);

        for field_addr in field_addrs.into_iter()? {
            let field = PField::new(self.process, self.name_manager.clone(), field_addr.into());
            let flags = field.flags()?;
            let is_static = match flags.contains(PFieldFlags::Static) {
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

bitflags! {
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct PFieldFlags: u32 {
        const Optional = 1 << 0;        // func param is optional
        const Method = 1 << 1;          // func has an implied self parameter
        const Action = 1 << 2;          // func has implied owner and state parameters
        const Native = 1 << 3;          // func is native code, field is natively defined
        const ReadOnly = 1 << 4;        // field is read only, do not write to it
        const Private = 1 << 5;         // field is private to containing class
        const Protected = 1 << 6;       // field is only accessible by containing class and children.
        const Deprecated = 1 << 7;      // Deprecated fields should output warnings when used.
        const Virtual = 1 << 8;         // function is virtual
        const Final = 1 << 9;           // Function may not be overridden in subclasses
        const In = 1 << 10;
        const Out = 1 << 11;
        const Implicit = 1 << 12;       // implicitly created parameters (i.e. do not compare types when checking function signatures)
        const Static = 1 << 13;
        const InternalAccess = 1 << 14; // overrides ReadOnly for internal script code.
        const Override = 1 << 15;       // overrides a virtual function from the parent class.
        const Ref = 1 << 16;            // argument is passed by reference.
        const Transient = 1 << 17;      // don't auto serialize field.
        const Meta = 1 << 18;           // static class data (by necessity read only.)
        const VarArg = 1 << 19;         // [ZZ] vararg: don't typecheck values after ... in function signature
        const UI = 1 << 20;             // [ZZ] ui: object is ui-scope only (can't modify playsim)
        const Play = 1 << 21;           // [ZZ] play: object is playsim-scope only (can't access ui)
        const VirtualScope= 1 << 22;    // [ZZ] virtualscope: object should use the scope of the particular class it's being used with (methods only)
        const ClearScope = 1 << 23;     // [ZZ] clearscope: this method ignores the member access chain that leads to it and is always plain data.
    }
}

#[derive(Clone)]
pub struct PField<'a> {
    process: &'a Process,
    name_manager: Rc<NameManager<'a>>,
    address: Address,

    name: OnceCell<String>,
    offset: OnceCell<u32>,
    ptype: OnceCell<PType<'a>>,
    flags: OnceCell<PFieldFlags>,
}

impl<'a> PField<'a> {
    pub fn new(
        process: &'a Process,
        name_manager: Rc<NameManager<'a>>,
        address: Address,
    ) -> PField<'a> {
        PField {
            process,
            name_manager,
            address,
            name: OnceCell::new(),
            offset: OnceCell::new(),
            ptype: OnceCell::new(),
            flags: OnceCell::new(),
        }
    }

    pub fn name(&self) -> Result<&String, Error> {
        self.name.get_or_try_init(|| {
            let name_index: u32 = self.process.read(self.address + PFIELD_NAME)?;

            self.name_manager.get_chars(name_index)
        })
    }

    pub fn offset(&self) -> Result<&u32, Error> {
        self.offset
            .get_or_try_init(|| self.process.read(self.address + PFIELD_OFFSET))
    }

    pub fn ptype(&self) -> Result<&PType, Error> {
        self.ptype.get_or_try_init(|| {
            let ptype_address: Address =
                self.process.read::<u64>(self.address + PFIELD_TYPE)?.into();

            Ok(PType::new(self.process, ptype_address))
        })
    }

    pub fn flags(&self) -> Result<&PFieldFlags, Error> {
        self.flags.get_or_try_init(|| {
            Ok(PFieldFlags::from_bits_truncate(
                self.process.read::<u32>(self.address + PFIELD_FLAGS)?,
            ))
        })
    }
}

impl<'a> PartialEq for PField<'a> {
    fn eq(&self, other: &Self) -> bool {
        let flags = self.flags();
        let other_flags = other.flags();
        if flags.is_err() || other_flags.is_err() {
            return false;
        }

        let name = self.name();
        let other_name = other.name();
        if name.is_err() || other_name.is_err() {
            return false;
        }

        return name.unwrap() == other_name.unwrap() || flags.unwrap() == other_flags.unwrap();
    }
}

impl<'a> Eq for PField<'a> {}

impl<'a> PartialOrd for PField<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for PField<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        let flags = self.flags().map(|f| f.to_owned()).unwrap_or_default();
        let other_flags = other.flags().map(|f| f.to_owned()).unwrap_or_default();

        if flags.contains(PFieldFlags::Static) && !other_flags.contains(PFieldFlags::Static) {
            return Ordering::Less;
        } else if !flags.contains(PFieldFlags::Static) && other_flags.contains(PFieldFlags::Static)
        {
            return Ordering::Greater;
        }

        let offset = self.offset().map(|o| o.to_owned()).unwrap_or_default();
        let other_offset = other.offset().map(|o| o.to_owned()).unwrap_or_default();

        if offset < other_offset {
            return Ordering::Less;
        } else if offset > other_offset {
            return Ordering::Greater;
        }

        return Ordering::Equal;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct TypeFlags: u32 {
        const Scalar = 1 << 0;
        const Container = 1 << 1;
        const Int = 1 << 2;
        const IntNotInt = 1 << 3;
        const Float = 1 << 4;
        const Pointer = 1 << 5;
        const ObjectPointer = 1 << 6;
        const ClassPointer = 1 << 7;
        const Array = 1 << 8;
    }
}

#[derive(Clone)]
pub struct PType<'a> {
    process: &'a Process,
    pub address: Address,

    size: OnceCell<u32>,
    align: OnceCell<u32>,
    flags: OnceCell<TypeFlags>,
    name: OnceCell<String>,
}

impl<'a> PType<'a> {
    pub fn new(process: &'a Process, address: Address) -> PType {
        PType {
            process,
            address,
            size: OnceCell::new(),
            align: OnceCell::new(),
            flags: OnceCell::new(),
            name: OnceCell::new(),
        }
    }

    pub fn size(&self) -> Result<&u32, Error> {
        self.size
            .get_or_try_init(|| self.process.read::<u32>(self.address.add(PTYPE_SIZE)))
    }

    pub fn align(&self) -> Result<&u32, Error> {
        self.align
            .get_or_try_init(|| self.process.read::<u32>(self.address.add(PTYPE_ALIGN)))
    }

    pub fn flags(&self) -> Result<&TypeFlags, Error> {
        self.flags.get_or_try_init(|| {
            Ok(TypeFlags::from_bits_truncate(
                self.process.read(self.address + PTYPE_FLAGS)?,
            ))
        })
    }

    pub fn name(&self) -> Result<&String, Error> {
        self.name.get_or_try_init(|| {
            let c_str = self.process.read_pointer_path::<ArrayCString<128>>(
                self.address,
                asr::PointerSize::Bit64,
                &[PTYPE_DESCRIPTIVE_NAME, 0x0],
            );

            let b = c_str?
                .validate_utf8()
                .expect("name should always be utf-8")
                .to_owned();

            Ok(b)
        })
    }
}
