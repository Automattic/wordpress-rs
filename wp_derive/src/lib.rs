use proc_macro::TokenStream;

mod wp_deserialize;
mod wp_messages;

#[proc_macro_derive(WpDeserialize)]
pub fn wp_deserialize(input: TokenStream) -> TokenStream {
    wp_deserialize::derive(input)
}

#[proc_macro_attribute]
pub fn wp_messages(_args: TokenStream, input: TokenStream) -> TokenStream {
    wp_messages::wp_messages(input)
}
