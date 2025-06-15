use std::cell::OnceCell;

#[derive(Debug)]
pub struct ClassTypeInfo<'a> {
    name: OnceCell<&'a str>,
    size: OnceCell<u32>,
}
