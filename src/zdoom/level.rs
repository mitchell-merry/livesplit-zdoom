use asr::{string::ArrayCString, Address, Error, Process};

// const LEVEL_MAPNAME_OFFSET: u64 = 0x2C8;
const LEVEL_MAPNAME_OFFSET: u64 = 0x9D8;

pub struct Level<'a> {
    process: &'a Process,
    address: Address,
    _name: Option<String>,
}

impl<'a> Level<'a> {
    pub fn new(process: &'a Process, address: Address) -> Level<'a> {
        Level {
            process,
            address,
            _name: None,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self._name = None
    }

    pub fn name(&mut self) -> Result<&str, Error> {
        if let Some(ref name) = self._name {
            return Ok(name);
        }

        let c_str = self.process.read_pointer_path::<ArrayCString<128>>(
            self.address,
            asr::PointerSize::Bit64,
            &[LEVEL_MAPNAME_OFFSET, 0x0],
        )?;

        let name = c_str
            .validate_utf8()
            .expect("name should always be utf-8")
            .to_owned();

        self._name = Some(name.clone());

        Ok(self._name.as_ref().unwrap())
    }
}
