#[wp_derive::wp_messages]
pub struct Messages {}

fn main() {
    let messages = Messages {};
    assert_eq!(Messages::foo(), "foo".to_string());
}
