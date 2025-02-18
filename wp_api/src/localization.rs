//#[allow(clippy::all)]
//use fluent_static::MessageBundle;
//
//#[fluent_static::message_bundle(
//    resources = [
//        ("localization/en-US/main.ftl", "en-US"),
//    ],
//    default_language = "en-US"
//)]
//pub struct Messages;

#[wp_derive::wp_translations]
pub struct Translations {}

//#[uniffi::export(with_foreign)]
//pub trait WpLocalizedError: Send + Sync {
//    fn localized_error_message(&self, locale_id: String) -> String;
//}
//
//#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
//pub enum FooError {
//    #[error("{}", Messages::default().foo_error_bar())]
//    Bar,
//    #[error("{}", Messages::default().foo_error_baz(value))]
//    Baz { value: String },
//    #[error("{}", Messages::default().foo_error_bazzz(value1, value2))]
//    Bazzz { value1: String, value2: String },
//}
//
//impl WpLocalizedError for FooError {
//    fn localized_error_message(&self, locale_id: String) -> String {
//        let messages = Messages::get(&locale_id).unwrap_or_default();
//        match self {
//            Self::Bar => messages.foo_bar().to_string(),
//            Self::Baz { value } => messages.foo_error_baz(value).to_string(),
//            Self::Bazzz { value1, value2 } => messages.foo_error_bazzz(value1, value2).to_string(),
//        }
//    }
//}
//
//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    #[test]
//    fn test_foo_error() {
//        assert_eq!(FooError::Bar.to_string(), "Foo is bar");
//        assert_eq!(
//            FooError::Baz {
//                value: "custom_baz".to_string()
//            }
//            .to_string(),
//            "Foo is \u{2068}custom_baz\u{2069}"
//        );
//        assert_eq!(
//            FooError::Bazzz {
//                value1: "custom_bazzz1".to_string(),
//                value2: "custom_bazzz2".to_string()
//            }
//            .to_string(),
//            "Foo is \u{2068}custom_bazzz1\u{2069} & \u{2068}custom_bazzz2\u{2069}"
//        );
//    }
//}
