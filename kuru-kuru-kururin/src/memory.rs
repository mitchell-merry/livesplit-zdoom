use crate::{GameFlags, GameMode};
use asr::emulator::gba::Emulator;
use bitflags::bitflags;
use bytemuck::{CheckedBitPattern, Pod, Zeroable};
use helpers::pointer::{Invalidatable, MemoryWatcher, PointerPath};

bitflags! {
    // this is available at 3000dec
    // not sure i have a use for this
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
    #[repr(C)]
    pub struct InputFlags: u16 {
        const A = 1 << 0;
        const B = 1 << 1;
        const Select = 1 << 2;
        const Start = 1 << 3;
        const Right = 1 << 4;
        const Left = 1 << 5;
        const Up = 1 << 6;
        const Down = 1 << 7;
        const R = 1 << 8;
        const L = 1 << 9;
    }
}

// no use for this atm
// this one is available at 0x3000dca
// there's a sub-menu at 0x3000dcb (byte)
#[derive(CheckedBitPattern, Clone, Copy, Default, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum GameState {
    #[default]
    None = 0,
    // sub-menus:
    // 0-2 initialisation / startup?
    // 3 cutscene at the beginning that loops
    // 4 transition to title screen
    // 5 title screen
    PressStart = 1,
    // sub-menus:
    // 1 save select
    // 2 enter save name screen
    SaveFiles = 2,
    // 0-1 mode select (adventure / practice / etc) + the make-up screen
    // 2 level select
    // 3 in level
    // 4 level win (value doesn't go back to 2 when you go back to level select)
    // 5 challenge select
    // 6 challenge level
    // 7 challenge level complete
    // 11 practice select
    // 12 practice level
    Game = 3,
}

pub struct Watchers<'a> {
    pub world: MemoryWatcher<'a, Emulator, u8>,
    pub sub_level: MemoryWatcher<'a, Emulator, u8>,
    pub game_mode: MemoryWatcher<'a, Emulator, GameMode>,
    pub time: MemoryWatcher<'a, Emulator, u32>,
    pub flags: MemoryWatcher<'a, Emulator, GameFlags>,
    pub input_flags: MemoryWatcher<'a, Emulator, InputFlags>,
    pub state: MemoryWatcher<'a, Emulator, GameState>,
    pub substate: MemoryWatcher<'a, Emulator, u8>,
}

impl<'a> Watchers<'a> {
    pub fn init(emulator: &'a Emulator) -> Self {
        let base = PointerPath::new32(emulator, 0x3004420_u64, &[]);
        // some more things:
        // 0x4 - save file slot

        // probably some instance of "level"?
        let some_important_thing = base.child(&[0x18, 0x0]);

        let world: MemoryWatcher<_, _> = base.child(&[0x0]).named("world").into();
        let sub_level: MemoryWatcher<_, _> = base.child(&[0x1]).named("sub level").into();
        let game_mode: MemoryWatcher<_, _> = base.child(&[0x16]).named("game mode").into();

        // 0x30059c0 + 0x24,0x26,0x34,0x28

        let time: MemoryWatcher<_, _> = some_important_thing.child(&[0xB8]).named("time").into();
        let flags: MemoryWatcher<_, _> = some_important_thing.child(&[0xBC]).named("flags").into();

        let state: MemoryWatcher<_, _> = PointerPath::new32(emulator, 0x3000dca_u64, &[])
            .named("state")
            .into();
        let substate: MemoryWatcher<_, _> = PointerPath::new32(emulator, 0x3000dcb_u64, &[])
            .named("substate")
            .into();
        let input_flags: MemoryWatcher<_, _> = PointerPath::new32(emulator, 0x3000dec_u64, &[])
            .named("buttons")
            .into();

        Watchers {
            world: world.default(),
            sub_level: sub_level.default(),
            game_mode: game_mode.default(),
            time: time.default(),
            flags: flags.default(),
            input_flags: input_flags.default(),
            state: state.default(),
            substate: substate.default(),
        }
    }

    pub fn invalidate(&mut self) {
        self.world.invalidate();
        self.sub_level.invalidate();
        self.game_mode.invalidate();
        self.time.invalidate();
        self.flags.invalidate();
        self.input_flags.invalidate();
    }
}
