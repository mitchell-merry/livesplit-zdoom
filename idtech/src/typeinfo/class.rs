use std::cell::OnceCell;

pub struct ClassTypeInfo<'a> {
    name: OnceCell<&'a str>,
    size: OnceCell<u32>,
}
