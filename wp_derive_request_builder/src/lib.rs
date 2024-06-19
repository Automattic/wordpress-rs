#![allow(unused)]
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod generate;
mod parse;
mod sparse_field_attr;
mod variant_attr;

#[proc_macro_derive(
    WpDerivedRequest,
    attributes(SparseField, get, post, delete, contextual_get)
)]
pub fn derive(input: TokenStream) -> TokenStream {
    let parsed_enum = parse_macro_input!(input as parse::ParsedEnum);

    if cfg!(feature = "generate_request_builder") {
        //dbg!("{:#?}", parsed_enum.clone());
        generate::generate_types(&parsed_enum)
            .unwrap_or_else(|err| err.into_compile_error())
            .into()
    } else {
        TokenStream::new()
    }
}
