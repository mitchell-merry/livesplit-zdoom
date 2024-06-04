use std::rc::Rc;

use crate::name_manager::NameManager;
use asr::{Address, Error, Process};
use bytemuck::CheckedBitPattern;
use once_cell::unsync::OnceCell;

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
    name_manager: Rc<NameManager<'a>>,
    address: Address,
    actor_class: PClass<'a>,
    actor: OnceCell<Address>,
    pos: OnceCell<DVector3>,
    state: OnceCell<PlayerState>,
}

impl<'a> Player<'a> {
    pub fn new(
        process: &'a Process,
        memory: Rc<Memory>,
        name_manager: Rc<NameManager<'a>>,
        address: Address,
        actor_class: PClass<'a>,
    ) -> Self {
        Player {
            process,
            memory,
            name_manager,
            address,
            actor_class,
            actor: OnceCell::new(),
            pos: OnceCell::new(),
            state: OnceCell::new(),
        }
    }

    fn actor(&self) -> Result<&Address, Error> {
        self.actor
            .get_or_try_init(|| Ok(self.process.read::<u64>(self.address)?.into()))
    }

    pub fn state(&self) -> Result<&PlayerState, Error> {
        self.state
            .get_or_try_init(|| self.process.read(self.address + PLAYER_STATE_OFFSET))
    }

    pub fn pos(&self) -> Result<&DVector3, Option<Error>> {
        self.pos.get_or_try_init(|| {
            let pos_field = self.actor_class.fields()?.get("pos");
            if pos_field.is_none() {
                return Err(None);
            }

            let actor = self.actor()?.to_owned();

            Ok(DVector3::read(
                self.process,
                actor + pos_field.unwrap().offset()?.to_owned(),
            )?)
        })
    }

    pub fn dump_inventories(&self) -> Result<(), Option<Error>> {
        let actor = self.actor()?.to_owned();
        let inv_offset = self
            .actor_class
            .fields()?
            .get("Inv")
            .ok_or(None)?
            .offset()?
            .to_owned();
        let mut inv: Address = self.process.read::<u64>(actor + inv_offset)?.into();
        while inv != Address::NULL {
            let class = self.process.read::<u64>(inv + 0x8)?.into();
            let class = PClass::new(
                self.process,
                self.memory.clone(),
                self.name_manager.clone(),
                class,
            );

            let name = class.name()?;
            asr::print_message(&format!("{name}, {inv}"));

            if name == "Objectives" {}

            inv = self.process.read::<u64>(inv + inv_offset)?.into();
        }

        Ok(())
    }
}
