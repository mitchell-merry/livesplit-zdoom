use std::rc::Rc;

use asr::{Address, Error, Process};

use super::{pclass::PClass, Memory};

#[derive(Clone, Debug, Default)]
pub struct DVector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl DVector3 {
    pub fn read(process: &Process, address: Address) -> Result<Self, Error> {
        Ok(DVector3 {
            x: process.read(address + 0x0)?,
            y: process.read(address + 0x8)?,
            z: process.read(address + 0x10)?,
        })
    }
}

#[derive(Clone)]
pub struct Player<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    address: Address,
    actor_class: PClass<'a>,
    _pos: Option<DVector3>,
}

impl<'a> Player<'a> {
    pub fn new(
        process: &'a Process,
        memory: Rc<Memory>,
        address: Address,
        actor_class: PClass<'a>,
    ) -> Self {
        Player {
            process,
            memory,
            address,
            actor_class,
            _pos: None,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self._pos = None;
    }

    pub fn pos(&mut self) -> Result<&DVector3, Error> {
        if let Some(ref pos) = self._pos {
            return Ok(pos);
        }

        let pos = DVector3::read(
            self.process,
            self.address
                + self
                    .actor_class
                    .fields()?
                    .get("pos")
                    .unwrap_or_else(|| panic!("can't find the position field on the actor"))
                    .offset()?
                    .to_owned(),
        )?;
        self._pos = Some(pos.clone());

        return Ok(self._pos.as_ref().unwrap());
    }
}
