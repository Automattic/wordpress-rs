use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::env;
use syn::{DeriveInput, Ident, parse_macro_input};
use wp_localization_parser::TranslationEntry;

include!(concat!(
    env!("OUT_DIR"),
    "/generated_localization_contents.rs"
));

pub fn wp_messages(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_input = input.clone();
    let messages_ident = parse_macro_input!(input as DeriveInput).ident;
    let entries = wp_localization_parser::parse(LOCALIZATION_CONTENTS)
        .expect("Localization file should be parseable");

    proc_macro::TokenStream::from_iter([
        // Since attribute macros completely replace the input, we need to manually preserve it
        original_input,
        generate_bindings(messages_ident, entries).into(),
    ])
}

fn generate_bindings(
    messages_ident: Ident,
    entries: impl Iterator<Item = TranslationEntry>,
) -> TokenStream {
    let functions = entries.map(|e| {
        if e.placeables.is_empty() {
            generate_argless_function(&e)
        } else {
            generate_function(&e)
        }
    });
    quote! {
        impl #messages_ident<'_> {
            #(#functions)*
        }
    }
}

fn generate_argless_function(entry: &TranslationEntry) -> TokenStream {
    let function_name = translation_entry_key_as_function_name(entry);
    let entry_key = translation_entry_key_as_token_string(entry);
    let documentation = generate_documentation(entry);
    quote! {
        #documentation
        pub fn #function_name<'a>() -> crate::MessageBundle<'a> {
            crate::MessageBundle::new(#entry_key, None)
        }
    }
}

fn generate_documentation(entry: &TranslationEntry) -> TokenStream {
    let doc = &entry.documentation;
    quote! {
        #[doc = #doc]
    }
}

fn generate_function(entry: &TranslationEntry) -> TokenStream {
    let function_name = translation_entry_key_as_function_name(entry);
    let entry_key = translation_entry_key_as_token_string(entry);
    let documentation = generate_documentation(entry);
    let args = entry.placeables.iter().map(|placeable| {
        let placeable_ident = format_ident!("{}", placeable.0);
        quote! {
            #placeable_ident: impl Into<fluent_bundle::FluentValue<'a>>,
        }
    });
    let map_inserts = entry.placeables.iter().map(|placeable| {
        let placeable_string = placeable.0;
        let placeable_ident = format_ident!("{}", placeable_string);
        quote! {
            map.insert(std::borrow::Cow::Borrowed(#placeable_string), #placeable_ident.into());
        }
    });
    quote! {
        #documentation
        pub fn #function_name<'a>(#(#args)*) -> crate::MessageBundle<'a> {
            let map = {
                let mut map = std::collections::HashMap::new();
                #(#map_inserts)*
                map
            };
            crate::MessageBundle::new(#entry_key, Some(map))
        }
    }
}

fn translation_entry_key_as_function_name(entry: &TranslationEntry) -> Ident {
    format_ident!("{}", entry.key.to_case(Case::Snake))
}

fn translation_entry_key_as_token_string(entry: &TranslationEntry) -> String {
    format_ident!("{}", entry.key).to_string()
}
