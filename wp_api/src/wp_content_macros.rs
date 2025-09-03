//! Macros for generating WordPress content ID and identifier types
//! These macros reduce duplication across ID types like PostId, MediaId, CategoryId, etc.

/// Generates a WordPress content ID type based on i64 with standard implementations
///
/// This macro creates:
/// - The ID struct with uniffi support
/// - AsQueryValue implementation  
/// - FromStr, Display, and From implementations
///
/// # Example
/// ```rust
/// # #[macro_use] extern crate wp_api;
/// # // We need these 2 lines for UniFFI
/// # uniffi::setup_scaffolding!();
/// # fn main() {}
/// wp_api::wp_content_i64_id!(PostId);
/// // Generates: PostId(i64) with all required traits
/// ```
#[macro_export]
macro_rules! wp_content_i64_id {
    ($id_type:ident) => {
        $crate::impl_as_query_value_for_new_type!($id_type);
        ::uniffi::custom_newtype!($id_type, i64);
        #[derive(Debug, Clone, Copy, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
        pub struct $id_type(pub i64);

        impl ::core::str::FromStr for $id_type {
            type Err = ::core::num::ParseIntError;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                s.parse().map(Self)
            }
        }

        impl ::core::fmt::Display for $id_type {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<i64> for $id_type {
            fn from(value: i64) -> Self {
                Self(value)
            }
        }
    };
}

/// Generates a WordPress content identifier type based on String with standard implementations
///
/// This macro creates:
/// - The identifier struct with uniffi support  
/// - AsQueryValue implementation using to_string
/// - FromStr (infallible), Display, and From implementations
///
/// # Example
/// ```rust
/// # #[macro_use] extern crate wp_api;
/// # // We need these 2 lines for UniFFI
/// # uniffi::setup_scaffolding!();
/// # fn main() {}
/// wp_api::wp_content_string_id!(WidgetId);
/// // Generates: WidgetId(String) with all required traits
/// ```
#[macro_export]
macro_rules! wp_content_string_id {
    ($id_type:ident) => {
        ::uniffi::custom_newtype!($id_type, ::std::string::String);
        $crate::impl_as_query_value_from_to_string!($id_type);
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
        pub struct $id_type(pub ::std::string::String);

        impl ::core::str::FromStr for $id_type {
            type Err = ::core::convert::Infallible;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                ::core::result::Result::Ok(Self(s.to_string()))
            }
        }

        impl ::core::fmt::Display for $id_type {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<::std::string::String> for $id_type {
            fn from(value: ::std::string::String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $id_type {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }
    };
}
