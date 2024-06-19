use std::fmt::Display;

use helpers_to_generate_tokens::*;
use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use syn::Ident;

use crate::{
    parse::{ParsedEnum, ParsedVariant, RequestType},
    sparse_field_attr::SparseFieldAttr,
};

mod helpers_to_generate_tokens;

pub(crate) fn generate_types(parsed_enum: &ParsedEnum) -> syn::Result<TokenStream> {
    let config = Config::new(parsed_enum);

    Ok(generate_endpoint_type(&config, &parsed_enum))
}

fn generate_endpoint_type(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let api_base_url_type = &config.api_base_url_type;
    let endpoint_ident = format_ident!("{}Endpoint", parsed_enum.enum_ident);

    let functions = parsed_enum.variants.iter().map(|variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;
        let request_type = variant.attr.request_type;
        let url_from_endpoint = fn_body_get_url_from_api_base_url(url_parts);
        let query_pairs = fn_body_query_pairs(params_type, request_type);

        ContextAndFilterHandler::from_request_type(request_type)
            .into_iter()
            .map(|context_and_filter_handler| {
                let fn_signature = fn_signature(
                    PartOf::Endpoint,
                    &variant.variant_ident,
                    url_parts,
                    params_type,
                    request_type,
                    context_and_filter_handler,
                    &config.sparse_field_type,
                );
                let context_query_pair =
                    fn_body_context_query_pairs(&config.crate_ident, context_and_filter_handler);
                let api_endpoint_url_type = &config.api_endpoint_url_type;
                quote! {
                    pub #fn_signature -> #api_endpoint_url_type {
                        #url_from_endpoint
                        #context_query_pair
                        #query_pairs
                        url.into()
                    }
                }
            })
            .collect::<TokenStream>()
    });

    quote! {
        #[derive(Debug)]
        pub struct #endpoint_ident {
            api_base_url: #api_base_url_type,
        }

        impl #endpoint_ident {
            pub fn new(api_base_url: #api_base_url_type) -> Self {
                Self { api_base_url }
            }

            #(#functions)*
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PartOf {
    Endpoint,
    RequestBuilder,
}

#[derive(Debug, Clone, Copy)]
pub enum ContextAndFilterHandler {
    None,
    NoFilterTakeContextAsArgument,
    NoFilterTakeContextAsFunctionName(WpContext),
    FilterTakeContextAsArgument,
    FilterNoContext,
}

impl ContextAndFilterHandler {
    fn from_request_type(request_type: RequestType) -> Vec<Self> {
        match request_type {
            crate::parse::RequestType::ContextualGet => {
                let mut v: Vec<Self> = WpContext::iter()
                    .map(|c| Self::NoFilterTakeContextAsFunctionName(c))
                    .collect();
                v.push(ContextAndFilterHandler::FilterTakeContextAsArgument);
                v
            }
            crate::parse::RequestType::Get => {
                vec![
                    ContextAndFilterHandler::NoFilterTakeContextAsArgument,
                    ContextAndFilterHandler::FilterTakeContextAsArgument,
                ]
            }
            crate::parse::RequestType::Delete | crate::parse::RequestType::Post => {
                vec![
                    ContextAndFilterHandler::None,
                    ContextAndFilterHandler::FilterNoContext,
                ]
            }
        }
    }
}

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum WpContext {
    Edit,
    Embed,
    View,
}

impl Display for WpContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WpContext::Edit => "Edit",
            WpContext::Embed => "Embed",
            WpContext::View => "View",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub struct Config {
    pub crate_ident: Ident,
    pub api_base_url_type: TokenStream,
    pub api_endpoint_url_type: TokenStream,
    pub request_builder_type: TokenStream,
    pub endpoint_ident: Ident,
    pub request_builder_ident: Ident,
    pub sparse_field_type: SparseFieldAttr,
}

impl Config {
    fn new(parsed_enum: &ParsedEnum) -> Self {
        let crate_name = "wp_api";
        let found_crate = proc_macro_crate::crate_name(crate_name)
            .unwrap_or_else(|_| panic!("{} is not present in `Cargo.toml`", crate_name));

        let crate_ident = match found_crate {
            FoundCrate::Itself => format_ident!("crate"),
            FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        };
        let api_base_url_type =
            quote! { std::sync::Arc<#crate_ident::request::endpoint::ApiBaseUrl> };
        let api_endpoint_url_type = quote! { #crate_ident::request::endpoint::ApiEndpointUrl };
        let request_builder_type = quote! { std::sync::Arc<#crate_ident::request::RequestBuilder> };
        Self {
            crate_ident,
            api_base_url_type,
            api_endpoint_url_type,
            request_builder_type,
            endpoint_ident: format_ident!("{}Endpoint", parsed_enum.enum_ident),
            request_builder_ident: format_ident!("{}Builder", parsed_enum.enum_ident),
            sparse_field_type: parsed_enum.sparse_field_attr.clone(),
        }
    }
}
