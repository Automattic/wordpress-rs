use serde::Serialize;
use wp_derive::WpDeserialize;

#[derive(Serialize, WpDeserialize)]
#[serde(transparent)]
pub struct Foo {
    pub bar: Option<String>,
}

fn main() {}
