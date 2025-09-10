use proc_macro::TokenStream;

mod params_field;
mod wp_deserialize;

#[proc_macro_derive(WpDeserialize)]
pub fn derive_wp_deserialize(input: TokenStream) -> TokenStream {
    wp_deserialize::derive(input)
}

#[proc_macro_derive(WpDeriveParamsField, attributes(field_name))]
pub fn derive_params_field(input: TokenStream) -> TokenStream {
    params_field::derive(input)
}
