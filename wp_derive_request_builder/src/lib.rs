use std::{env, fs};

use generate::CrateConfig;
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod generate;
mod parse;
mod variant_attr;

#[proc_macro_derive(WpDerivedRequest, attributes(contextual_get, delete, get, post))]
pub fn derive(input: TokenStream) -> TokenStream {
    let parsed_enum = parse_macro_input!(input as parse::ParsedEnum);

    if cfg!(feature = "generate_request_builder") {
        let crate_config = read_crate_config();
        generate::generate_types(&parsed_enum, &crate_config).into()
    } else {
        TokenStream::new()
    }
}

fn read_crate_config() -> CrateConfig {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("Crate config can't be found without the `CARGO_MANIFEST_DIR` environment varible");
    let file_path = format!("{}/wp_derived_request.toml", manifest_dir);
    let contents = match fs::read_to_string(&file_path) {
        Ok(c) => toml::from_str(c.as_str())
            .unwrap_or_else(|e| panic!("'{}' is not formatted correctly:\n{:#?}", file_path, e)),
        Err(_) => {
            panic!("{} is missing", file_path);
        }
    };
    contents
}
