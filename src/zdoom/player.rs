use std::fmt::Display;

use asr::{Address, Error, Process};

const PLAYER_POS_OFFSET: u32 = 0x48;
// const PLAYER_POS_OFFSET: u32 = 0x50;

#[derive(Clone, Debug)]
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

pub struct Player<'a> {
    process: &'a Process,
    address: Address,
    _pos: Option<DVector3>,
}

impl<'a> Player<'a> {
    pub fn new(process: &'a Process, address: Address) -> Self {
        Player {
            process,
            address,
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

        let pos = DVector3::read(self.process, self.address + PLAYER_POS_OFFSET)?;
        self._pos = Some(pos.clone());

        return Ok(self._pos.as_ref().unwrap());
    }
}
