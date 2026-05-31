use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, parse2};

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream::from(input);
    let derive_input: DeriveInput = parse2(input).expect("Failed to parse input");

    let enum_name = &derive_input.ident;
    let value_name = format_ident!("{}Value", enum_name);

    // Check for #[parsed_value(resolution = "serde")] attribute to use serde-based resolution
    // instead of strum's FromStr/Display. Default is strum.
    let uses_serde = derive_input.attrs.iter().any(|attr| {
        if !attr.path().is_ident("parsed_value") {
            return false;
        }
        attr.parse_args::<syn::MetaNameValue>()
            .map(|nv| {
                nv.path.is_ident("resolution")
                    && matches!(&nv.value, syn::Expr::Lit(lit) if matches!(&lit.lit, syn::Lit::Str(s) if s.value() == "serde"))
            })
            .unwrap_or(false)
    });

    let from_raw_body = if uses_serde {
        quote! {
            let value = serde_json::from_value::<#enum_name>(
                serde_json::Value::String(raw.clone())
            ).ok();
            Self { inner_value: value, inner_raw: raw }
        }
    } else {
        quote! {
            use std::str::FromStr;
            let value = #enum_name::from_str(&raw).ok();
            // Filter out the Custom/fallback variant — if FromStr returns a Custom variant,
            // that means it wasn't a known variant, so value should be None.
            let value = value.filter(|v| v.to_string() == raw);
            Self { inner_value: value, inner_raw: raw }
        }
    };

    let from_value_body = if uses_serde {
        // For serde-based enums, we can't easily get the string from a variant at runtime
        // without serializing. We'll use serde_json for now.
        quote! {
            let raw = serde_json::to_value(&value)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            Self { inner_value: Some(value), inner_raw: raw }
        }
    } else {
        quote! {
            let raw = value.to_string();
            Self { inner_value: Some(value), inner_raw: raw }
        }
    };

    let output = quote! {
        #[derive(Debug, Clone, uniffi::Object)]
        #[uniffi::export(Eq, Hash)]
        pub struct #value_name {
            inner_value: Option<#enum_name>,
            inner_raw: String,
        }

        impl PartialEq for #value_name {
            fn eq(&self, other: &Self) -> bool {
                self.inner_raw == other.inner_raw
            }
        }

        impl Eq for #value_name {}

        impl std::hash::Hash for #value_name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.inner_raw.hash(state);
            }
        }

        impl std::fmt::Display for #value_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.inner_raw)
            }
        }

        impl std::str::FromStr for #value_name {
            type Err = std::convert::Infallible;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self::new_from_raw(s.to_string()))
            }
        }

        impl<'de> serde::Deserialize<'de> for #value_name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Ok(Self::new_from_raw(raw))
            }
        }

        impl serde::Serialize for #value_name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.inner_raw.serialize(serializer)
            }
        }

        impl From<#enum_name> for #value_name {
            fn from(value: #enum_name) -> Self {
                Self::new_from_value(value)
            }
        }

        #[uniffi::export]
        impl #value_name {
            /// Create from a raw API string. Resolves to the known enum variant if possible.
            #[uniffi::constructor]
            pub fn new_from_raw(raw: String) -> Self {
                #from_raw_body
            }

            /// Create from a known enum variant. Derives the raw string automatically.
            #[uniffi::constructor]
            pub fn new_from_value(value: #enum_name) -> Self {
                #from_value_body
            }

            /// The parsed enum variant, or `None` if the raw string is unknown.
            pub fn value(&self) -> Option<#enum_name> {
                self.inner_value.clone()
            }

            /// The original raw string from the API.
            pub fn raw(&self) -> String {
                self.inner_raw.clone()
            }

            /// Check if this matches a known enum variant.
            pub fn is_code(&self, code: #enum_name) -> bool {
                self.inner_raw == Self::new_from_value(code).inner_raw
            }

            /// Check if the raw string matches.
            pub fn is_raw(&self, raw: String) -> bool {
                self.inner_raw == raw
            }

            /// Check if this matches any of the given enum variants.
            pub fn is_any_code(&self, codes: Vec<#enum_name>) -> bool {
                codes.into_iter().any(|c| self.is_code(c))
            }
        }

        impl #value_name {
            /// Access the inner enum value (for Rust-side pattern matching).
            pub(crate) fn inner(&self) -> Option<&#enum_name> {
                self.inner_value.as_ref()
            }
        }
    };

    output.into()
}
