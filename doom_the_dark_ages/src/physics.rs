use helpers::pointer::{Invalidatable, MemoryWatcher, PointerPath};
use idtech::IdTech;
use std::error::Error;

pub struct IdVec3<'a> {
    pub x: MemoryWatcher<'a, f32>,
    pub y: MemoryWatcher<'a, f32>,
    pub z: MemoryWatcher<'a, f32>,
}

impl<'a> IdVec3<'a> {
    pub fn init(idtech: &IdTech<'a>, path: PointerPath<'a>) -> Result<Self, Box<dyn Error>> {
        let c = idtech.get_class("Engine", "idVec3")?;
        Ok(IdVec3 {
            x: path.child(&[c.get_offset("x")?]).into(),
            y: path.child(&[c.get_offset("y")?]).into(),
            z: path.child(&[c.get_offset("z")?]).into(),
        })
    }
}

impl<'a> Invalidatable for IdVec3<'a> {
    fn next_tick(&mut self) {
        self.x.next_tick();
        self.y.next_tick();
        self.z.next_tick();
    }
}
