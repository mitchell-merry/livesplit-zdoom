mod typeinfo;

use std::rc::Rc;
use std::time::Duration;

use asr::{print_message, signature::Signature, Address, Error, Process};
use bytemuck::CheckedBitPattern;
use typeinfo::*;

pub struct IdTech<'a> {
    process: &'a Process,
    pub memory: Rc<Memory>,
    pub type_info: Rc<TypeInfoTools<'a>>,
}

impl<'a> IdTech<'a> {
    pub async fn wait_try_load<T, F>(
        process: &'a Process,
        version: IdTechVersion,
        main_module_name: &str,
        load_fn: F,
    ) -> (IdTech<'a>, T)
    where
        F: Fn(&IdTech) -> Result<T, Option<Error>>,
    {
        asr::print_message(&format!("idtech: Using version {version:?}"));
        let cooldown = Duration::from_secs(3);

        let fail_action = || async {
            print_message(&format!(
                "try_load unsuccessful, waiting {}s...",
                cooldown.as_secs()
            ));
            asr::future::sleep(cooldown).await;
        };

        loop {
            let memory = Memory::new(process, version, main_module_name);
            if memory.is_err() {
                fail_action().await;
                continue;
            }

            let memory = Rc::new(memory.unwrap());

            let typeinfo_instance = process.read::<u64>(memory.typeinfo_addr);
            if typeinfo_instance.is_err() {
                fail_action().await;
                continue;
            }

            let typeinfo_instance = typeinfo_instance.unwrap().into();
            if (typeinfo_instance == Address::NULL) {
                fail_action().await;
                continue;
            }

            let type_info = Rc::new(TypeInfoTools::new(process, typeinfo_instance));
            let projects = type_info.projects();
            if projects.is_err() {
                fail_action().await;
                continue;
            }

            for i in projects.unwrap() {
                let name = i.name();
                if name.is_err() {
                    fail_action().await;
                    continue;
                }

                print_message(name.unwrap());
            }

            let idtech = IdTech {
                process,
                memory,
                type_info,
            };

            let result = load_fn(&idtech);
            if result.is_err() {
                print_message("try_load: error running load_fn");
                fail_action().await;
                continue;
            }

            print_message("try_load successful!");
            return (idtech, result.unwrap());
        }
    }

    pub fn invalidate_cache(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

// disclaimer: I don't know much about the different idtech versions work...
// i have only tried this with a few games
#[derive(Clone, Copy, Debug)]
pub enum IdTechVersion {
    IdTech8, // Doom: The Dark Ages
}

type ScanFn = fn(process: &Process, module_range: (Address, u64)) -> Result<Address, Option<Error>>;

fn find_addr_or_panic(
    name: &str,
    process: &Process,
    module_range: (Address, u64),
    sigs: Vec<ScanFn>,
) -> Address {
    for (i, sig) in sigs.iter().enumerate() {
        if let Ok(addr) = sig(process, module_range) {
            asr::print_message(&format!(
                "Found {name} at 0x{addr} with signature index {i}"
            ));
            return addr;
        }
    }

    panic!("unable to find addr for {name}");
}

fn scan<const N: usize>(
    signature: Signature<N>,
    process: &Process,
    (addr, len): (Address, u64),
    offset: u32,
    next_instruction: u32,
) -> Result<Address, Option<Error>> {
    let addr = signature
        .scan_process_range(process, (addr, len))
        .ok_or(None)?
        + offset;

    Ok(addr + process.read::<u32>(addr)? + next_instruction)
}

pub struct Memory {
    typeinfo_addr: Address,

    offsets: Offsets,
}

impl Memory {
    fn new(
        process: &Process,
        version: IdTechVersion,
        main_module_name: &str,
    ) -> Result<Memory, Error> {
        let module_range = process.get_module_range(main_module_name)?;

        let typeinfo_sigs: Vec<ScanFn> = vec![|p, mr| {
            scan(
                    Signature::<29>::new(
                        "48 8b fa 4c 89 41 08 48 8b d9 48 85 D2 74 25 48 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 89 03",
                    ),
                    p,
                    mr,
                    0x12,
                    0x4,
                )
        }];

        Ok(Memory {
            typeinfo_addr: find_addr_or_panic("typeinfo", process, module_range, typeinfo_sigs),
            offsets: Offsets::new(version),
        })
    }
}

struct Offsets {}

impl Offsets {
    fn new(version: IdTechVersion) -> Self {
        match version {
            IdTechVersion::IdTech8 => Self {},
        }
    }
}
