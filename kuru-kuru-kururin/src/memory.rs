use crate::{GameFlags, GameMode};
use asr::emulator::gba::Emulator;
use helpers::pointer::{Invalidatable, MemoryWatcher, PointerPath};

pub struct Watchers<'a> {
    pub world: MemoryWatcher<'a, Emulator, u8>,
    pub sub_level: MemoryWatcher<'a, Emulator, u8>,
    pub game_mode: MemoryWatcher<'a, Emulator, GameMode>,
    pub time: MemoryWatcher<'a, Emulator, u32>,
    pub flags: MemoryWatcher<'a, Emulator, GameFlags>,
}

impl<'a> Watchers<'a> {
    pub fn init(emulator: &'a Emulator) -> Self {
        let base = PointerPath::new32(emulator, 0x3004420_u64.into(), &[]);
        let some_important_thing = base.child(&[0x18, 0x0]);

        Watchers {
            world: base.child(&[0x0]).named("world").into(),
            sub_level: base.child(&[0x1]).named("sub level").into(),
            game_mode: base.child(&[0x16]).named("game mode").into(),
            time: some_important_thing.child(&[0xB8]).named("time").into(),
            flags: some_important_thing.child(&[0xBC]).named("flags").into(),
        }
    }

    pub fn invalidate(&mut self) {
        self.world.invalidate();
        self.sub_level.invalidate();
        self.game_mode.invalidate();
        self.time.invalidate();
        self.flags.invalidate();
    }
}
