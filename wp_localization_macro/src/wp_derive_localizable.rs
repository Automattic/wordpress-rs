use convert_case::{Case, Casing};
use proc_macro_crate::FoundCrate;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident, parse_macro_input};

pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let localization_crate_ident = crate_ident("wp_localization");
    let derive_input = parse_macro_input!(input as DeriveInput);
    let original_ident = derive_input.ident;
    let localize_function_ident = format_ident!(
        "localize_{}",
        original_ident.to_string().to_case(Case::Snake)
    );
    quote! {
        impl std::fmt::Display for #original_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.message_bundle())
            }
        }

        #[uniffi::export]
        fn #localize_function_ident(value: &#original_ident, locale: Option<#localization_crate_ident::WpLocale>) -> String {
            #localization_crate_ident::WpLocalizable::localize(value, locale)
        }
    }
    .into()
}

fn crate_ident(crate_name: &str) -> Ident {
    let found_crate = proc_macro_crate::crate_name(crate_name)
        .unwrap_or_else(|_| panic!("{} is not present in `Cargo.toml`", crate_name));
    match found_crate {
        FoundCrate::Itself => format_ident!("crate"),
        FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
    }
}
