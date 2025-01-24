use serde::Serialize;
use wp_derive::WpDeserialize;

#[derive(Serialize, WpDeserialize)]
pub struct Foo {
    pub bar: Vec<u32>,
}

fn main() {}
