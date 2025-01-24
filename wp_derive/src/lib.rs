use proc_macro::TokenStream;

mod wp_deserialize;

#[proc_macro_derive(WpDeserialize)]
pub fn derive(input: TokenStream) -> TokenStream {
    wp_deserialize::derive(input)
}
