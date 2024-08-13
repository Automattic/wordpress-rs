use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn immutable_test(input: TokenStream, annotated_item: TokenStream) -> TokenStream {
    println!("input: {:#?}", input);
    println!("annotated_item: {:#?}", annotated_item);
    annotated_item
}
