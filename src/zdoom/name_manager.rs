use asr::{string::ArrayCString, Address, Error, Process};

const NAME_ENTRY_SIZE: u64 = 0x10;

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
        )?;

        match read.validate_utf8() {
            Ok(s) => Ok(s.to_owned()),
            Err(e) => {
                asr::print_message(&format!("what? {e}"));
                panic!("waaaah..");
            }
        }
    }
}
