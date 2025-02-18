#[wp_derive::wp_translations]
pub struct Translations {}

fn main() {
    let translations = Translations {};
    assert_eq!(Translations::foo(), "foo".to_string());
}
