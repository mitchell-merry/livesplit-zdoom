// Please don't go after me idSoftware
// I like you :)
use crate::typeinfo::class::ClassTypeInfo;
use asr::string::ArrayCString;
use asr::{Address, Error, Process};
use once_cell::unsync::OnceCell;

pub mod class;

// idArray < idTypeInfoTools::registeredTypeInfo_t , 2 > generatedTypeInfo; // 0x00000 (size: 0x70) -
const TYPE_INFO_TOOLS_GENERATED_TYPE_INFO_OFFSET: u64 = 0x0;

const TYPE_INFO_PROJECT_SIZE: u64 = 0x38;
// typeInfoGenerated_t typeInfoGenerated;  // 0x00000 (size: 0x8) - type info generated with the TypeInfoGen
const TYPE_INFO_PROJECT_TYPE_INFO_GENERATED_OFFSET: u64 = 0x0;
// char projectName; // 0x00000 (size: 0x8) -
const TYPE_INFO_PROJECT_NAME_OFFSET: u64 = 0x0;

/** Mirrors the idTypeInfoTools class */
pub struct TypeInfoTools<'a> {
    process: &'a Process,
    address: Address,

    projects: OnceCell<Vec<TypeInfoProject<'a>>>,
}

impl<'a> TypeInfoTools<'a> {
    pub fn new(process: &'a Process, address: Address) -> TypeInfoTools<'a> {
        TypeInfoTools {
            process,
            address,

            projects: OnceCell::new(),
        }
    }

    pub fn projects(&self) -> Result<&Vec<TypeInfoProject<'a>>, Error> {
        self.projects.get_or_try_init(|| {
            let mut projects = Vec::new();

            let mut current_project =
                self.address.clone() + TYPE_INFO_TOOLS_GENERATED_TYPE_INFO_OFFSET;
            // TODO: the range should be dynamic?
            for i in 0..2 {
                projects.push(TypeInfoProject::new(&self.process, current_project.clone()));
                current_project = current_project + TYPE_INFO_PROJECT_SIZE;
            }

            Ok(projects)
        })
    }
}

/** Mirrors the idTypeInfoTools::registeredTypeInfo_t class, which has a projectName */
pub struct TypeInfoProject<'a> {
    process: &'a Process,
    address: Address,

    name: OnceCell<String>,
    classes: OnceCell<Vec<ClassTypeInfo<'a>>>,
}

impl<'a> TypeInfoProject<'a> {
    pub fn new(process: &'a Process, address: Address) -> TypeInfoProject<'a> {
        TypeInfoProject {
            process,
            address,

            name: OnceCell::new(),
            classes: OnceCell::new(),
        }
    }

    pub fn name(&self) -> Result<&String, Error> {
        self.name.get_or_try_init(|| {
            Ok(self
                .process
                .read_pointer_path::<ArrayCString<512>>(
                    self.address,
                    asr::PointerSize::Bit64,
                    &[
                        TYPE_INFO_PROJECT_TYPE_INFO_GENERATED_OFFSET,
                        TYPE_INFO_PROJECT_NAME_OFFSET,
                        0x0,
                    ],
                )?
                .validate_utf8()
                .expect("title should always be utf-8")
                .to_owned())
        })
    }
}
