use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let original_ident = derive_input.ident;
    let snake_case_ident = format_ident!("{}", original_ident.to_string().to_case(Case::Snake));
    quote! {
        impl std::fmt::Display for #original_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.message_bundle())
            }
        }

        use crate::UniffiLocalizable;

        #[uniffi::export]
        impl UniffiLocalizable {
            #[uniffi::constructor]
            fn #snake_case_ident(value: #original_ident) -> Self {
                Self(std::sync::Arc::new(value))
            }
        }
    }
    .into()
}
