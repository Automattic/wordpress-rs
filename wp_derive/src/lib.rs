use proc_macro::TokenStream;
use syn::parse_macro_input;

mod wp_contextual;

#[proc_macro_derive(WPContextual, attributes(WPContext, WPContextualField))]
pub fn derive(input: TokenStream) -> TokenStream {
    wp_contextual::wp_contextual(parse_macro_input!(input))
        .unwrap_or_else(|err| err.into_compile_error().into())
}
