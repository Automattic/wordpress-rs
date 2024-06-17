#![allow(unused)]
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod parse;
mod sparse_field_attr;
mod variant_attr;

#[proc_macro_derive(
    WpDerivedRequest,
    attributes(SparseField, get, post, delete, contextual_get)
)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as parse::ParsedEnum);

    if cfg!(feature = "generate_request_builder") {
        dbg!("{:#?}", input);
        // TODO: Generate endpoint & request builder
        TokenStream::new()
    } else {
        TokenStream::new()
    }
}
