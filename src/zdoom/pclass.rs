use std::collections::HashMap;

use asr::{string::ArrayCString, Address, Error, Process};
use bitflags::bitflags;

use super::{name_manager::{self, NameManager}, tarray::TArray};

const PFIELD_NAME: u64 = 0x28;
const PFIELD_OFFSET: u64 = 0x38;
const PFIELD_TYPE: u64 = 0x40;
const PFIELD_FLAGS: u64 = 0x48;

const PCLASS_TYPENAME: u64 = 0x38;
const PCLASS_DESCRIPTIVE_NAME: u64 = 0x88;

pub struct PClass<'a> {
    process: &'a Process,
    address: Address,
}

impl<'a> PClass<'a> {
    pub fn new(process: &'a Process, addr: Address) -> PClass<'a> {
        PClass {
            process,
            address: addr,
        }
    }

    pub fn name(&self, name_manager: &NameManager) -> Result<String, Error> {
        return name_manager
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

    pub fn debug_all_fields(&self, name_manager: &NameManager) -> Result<(), Error> {
        let parent_class: Address = self
            .process
            .read_pointer_path::<u64>(self.address, asr::PointerSize::Bit64, &[0x0])?
            .into();

        if parent_class != Address::NULL {
            let parent_class = PClass::new(self.process, parent_class);
            parent_class.debug_all_fields(name_manager)?;
        }

        asr::print_message(&format!("{} fields:", self.name(name_manager)?));

        let fields_addr = self.address.add(0x78);
        let field_addrs = TArray::<u64>::new(self.process, fields_addr.into());

        for field_addr in field_addrs.into_iter()? {
            let field = PField::new(&self.process, field_addr.into());
            let flags = field.flags()?;
            let is_static = match flags.contains(PFieldFlags::VARF_Static) {
                true => " static",
                false => "       ",
            };

            asr::print_message(&format!(
                "  => {is_static} 0x{:X}  {}: {} [{:?}]",
                field.offset()?,
                field.name(name_manager)?,
                field.ptype()?.name()?,
                flags
            ));
        }

        Ok(())
    }
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
    address: Address,
}

impl<'a> PField<'a> {
    pub fn new(process: &'a Process, address: Address) -> PField<'a> {
        PField {
            process,
            address,
        }
    }

    pub fn name(&self, name_manager: &NameManager) -> Result<String, Error> {
        let name_index: u32 = self.process.read_pointer_path(
            self.address,
            asr::PointerSize::Bit64,
            &[PFIELD_NAME],
        )?;

        name_manager.get_chars(name_index)
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
