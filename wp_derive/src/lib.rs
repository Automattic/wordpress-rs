use proc_macro::TokenStream;

mod wp_deserialize;
mod wp_translations;

#[proc_macro_derive(WpDeserialize)]
pub fn wp_deserialize(input: TokenStream) -> TokenStream {
    wp_deserialize::derive(input)
}

#[proc_macro_attribute]
pub fn wp_translations(_args: TokenStream, input: TokenStream) -> TokenStream {
    wp_translations::wp_translations(input)
}
