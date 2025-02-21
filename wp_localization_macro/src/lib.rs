use proc_macro::TokenStream;

mod wp_messages;

#[proc_macro_attribute]
pub fn wp_messages(_args: TokenStream, input: TokenStream) -> TokenStream {
    wp_messages::wp_messages(input)
}
