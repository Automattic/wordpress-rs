use proc_macro::TokenStream;

mod wp_derive_localizable;
mod wp_messages;

#[proc_macro_derive(WpDeriveLocalizable)]
pub fn derive(input: TokenStream) -> TokenStream {
    wp_derive_localizable::derive(input)
}

#[proc_macro_attribute]
pub fn wp_messages(_args: TokenStream, input: TokenStream) -> TokenStream {
    wp_messages::wp_messages(input)
}
