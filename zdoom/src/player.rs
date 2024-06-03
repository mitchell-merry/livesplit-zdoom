use std::rc::Rc;

use asr::{Address, Error, Process};
use bytemuck::CheckedBitPattern;

use super::{pclass::PClass, Memory};

const PLAYER_ACTOR_OFFSET: u64 = 0x0;
const PLAYER_STATE_OFFSET: u64 = 0x8;

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

#[derive(CheckedBitPattern, Clone, Copy, Default, Debug, PartialEq)]
#[repr(u32)]
pub enum PlayerState {
    // comments are from the source code
    #[default]
    Live, // Playing or camping.
    Dead,   // Dead on the ground, view follows killer.
    Reborn, // Ready to restart/respawn???
    Enter,  // [BC] Entered the game
    Gone,   // Player has left the game
}

#[derive(Clone)]
pub struct Player<'a> {
    process: &'a Process,
    memory: Rc<Memory>,
    address: Address,
    actor_class: PClass<'a>,
    _pos: Option<DVector3>,
    _state: Option<PlayerState>,
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
            _state: None,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self._pos = None;
        self._state = None;
    }

    pub fn state(&mut self) -> Result<PlayerState, Error> {
        if self._state.is_none() {
            self._state = Some(self.process.read(self.address + PLAYER_STATE_OFFSET)?);
        }

        Ok(self._state.unwrap())
    }

    pub fn pos(&mut self) -> Result<&DVector3, Option<Error>> {
        if let Some(ref pos) = self._pos {
            return Ok(pos);
        }

        let pos_field = self.actor_class.fields()?.get("pos");

        if pos_field.is_none() {
            return Err(None);
        }

        let actor_addr: Address = self.process.read::<u64>(self.address)?.into();

        let pos = DVector3::read(
            self.process,
            actor_addr + pos_field.unwrap().offset()?.to_owned(),
        )?;
        self._pos = Some(pos.clone());

        Ok(self._pos.as_ref().unwrap())
    }
}
