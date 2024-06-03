use std::rc::Rc;

use asr::{string::ArrayCString, Address, Error, Process};

use super::Memory;

pub struct Level<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    address: Address,
    _name: Option<String>,
}

impl<'a> Level<'a> {
    pub fn new(process: &'a Process, memory: Rc<Memory>, address: Address) -> Level<'a> {
        Level {
            process,
            memory,
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
            &[self.memory.offsets.level_mapname, 0x0],
        )?;

        let name = c_str
            .validate_utf8()
            .expect("name should always be utf-8")
            .to_owned();

        self._name = Some(name.clone());

        Ok(self._name.as_ref().unwrap())
    }
}
