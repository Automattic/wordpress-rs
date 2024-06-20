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

pub(crate) fn generate_types(parsed_enum: &ParsedEnum) -> TokenStream {
    let config = Config::new(parsed_enum);
    TokenStream::from_iter(
        &mut [
            generate_endpoint_type(&config, parsed_enum),
            generate_request_builder(&config, parsed_enum),
            generate_async_request_executor(&config, parsed_enum),
        ]
        .into_iter(),
    )
}

fn generate_async_request_executor(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let api_base_url_type = &config.api_base_url_type;
    let request_builder_ident = &config.request_builder_ident;
    let request_executor_ident = &config.request_executor_ident;
    let request_executor_type = &config.request_executor_type;
    let wp_api_error_type = &config.wp_api_error_type;

    let functions = parsed_enum.variants.iter().map(|variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;

        ContextAndFilterHandler::from_request_type(variant.attr.request_type)
            .into_iter()
            .map(|context_and_filter_handler| {
                let output_type =
                    output_type(variant.attr.output.clone(), context_and_filter_handler);
                let request_from_request_builder = fn_body_get_request_from_request_builder(
                    &variant.variant_ident,
                    url_parts,
                    params_type,
                    variant.attr.request_type,
                    context_and_filter_handler,
                );
                let fn_signature = fn_signature(
                    PartOf::RequestExecutor,
                    &variant.variant_ident,
                    url_parts,
                    params_type,
                    variant.attr.request_type,
                    context_and_filter_handler,
                    &config.sparse_field_type,
                );
                quote! {
                    pub async #fn_signature -> Result<#output_type, #wp_api_error_type> {
                        #request_from_request_builder
                        self.request_executor.execute(request).await?.parse()
                   }
                }
            })
            .collect::<TokenStream>()
    });

    quote! {
        #[derive(Debug, uniffi::Object)]
        pub struct #request_executor_ident {
            request_builder: #request_builder_ident,
            request_executor: #request_executor_type,
        }
        impl #request_executor_ident {
            pub(crate) fn new(request_builder: #request_builder_ident, request_executor: #request_executor_type) -> Self {
                Self {
                    request_builder,
                    request_executor,
                }
            }
        }
        #[uniffi::export]
        impl #request_executor_ident {
            #(#functions)*
        }
    }
}

fn generate_request_builder(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let api_base_url_type = &config.api_base_url_type;
    let endpoint_ident = &config.endpoint_ident;
    let request_builder_ident = &config.request_builder_ident;
    let request_builder_type = &config.request_builder_type;
    let wp_network_request_type = &config.wp_network_request_type;

    let functions = parsed_enum.variants.iter().map(|variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;

        ContextAndFilterHandler::from_request_type(variant.attr.request_type)
            .into_iter()
            .map(|context_and_filter_handler| {
                let url_from_endpoint = fn_body_get_url_from_endpoint(
                    &variant.variant_ident,
                    url_parts,
                    params_type,
                    variant.attr.request_type,
                    context_and_filter_handler,
                );
                let fn_signature = fn_signature(
                    PartOf::RequestBuilder,
                    &variant.variant_ident,
                    url_parts,
                    params_type,
                    variant.attr.request_type,
                    context_and_filter_handler,
                    &config.sparse_field_type,
                );
                let fn_body_build_request_from_url =
                    fn_body_build_request_from_url(params_type, variant.attr.request_type);
                quote! {
                    pub #fn_signature -> #wp_network_request_type {
                        #url_from_endpoint
                        #fn_body_build_request_from_url
                    }
                }
            })
            .collect::<TokenStream>()
    });

    quote! {
        #[derive(Debug, uniffi::Object)]
        pub struct #request_builder_ident {
            endpoint: #endpoint_ident,
            request_builder: #request_builder_type,
        }
        impl #request_builder_ident {
            pub(crate) fn new(api_base_url: #api_base_url_type, request_builder: #request_builder_type) -> Self {
                Self {
                    endpoint: #endpoint_ident::new(api_base_url),
                    request_builder,
                }
            }
        }
        #[uniffi::export]
        impl #request_builder_ident {
            #(#functions)*
        }
    }
}

fn generate_endpoint_type(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let api_base_url_type = &config.api_base_url_type;
    let endpoint_ident = &config.endpoint_ident;

    let functions = parsed_enum.variants.iter().map(|variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;
        let request_type = variant.attr.request_type;
        let url_from_api_base_url = fn_body_get_url_from_api_base_url(url_parts);
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
                let fields_query_pairs =
                    fn_body_fields_query_pairs(&config.crate_ident, context_and_filter_handler);
                quote! {
                    pub #fn_signature -> #api_endpoint_url_type {
                        #url_from_api_base_url
                        #context_query_pair
                        #query_pairs
                        #fields_query_pairs
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
    RequestExecutor,
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
                    .map(Self::NoFilterTakeContextAsFunctionName)
                    .collect();
                v.push(ContextAndFilterHandler::FilterTakeContextAsArgument);
                v
            }
            crate::parse::RequestType::Delete | crate::parse::RequestType::Post => {
                vec![ContextAndFilterHandler::None]
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
    // TODO: It's not clear what some of the names refer to and the difference between them
    // For example, with "request_builder_ident" & "request_builder_type"
    pub api_base_url_type: TokenStream,
    pub api_endpoint_url_type: TokenStream,
    pub crate_ident: Ident,
    pub endpoint_ident: Ident,
    pub request_builder_ident: Ident,
    pub request_builder_type: TokenStream,
    pub request_executor_ident: Ident,
    pub request_executor_type: TokenStream,
    pub sparse_field_type: SparseFieldAttr,
    pub wp_api_error_type: TokenStream,
    pub wp_network_request_type: TokenStream,
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
        let request_executor_type =
            quote! { std::sync::Arc<dyn #crate_ident::request::RequestExecutor> };
        let wp_api_error_type = quote! { #crate_ident::WpApiError };
        let wp_network_request_type = quote! { #crate_ident::request::WpNetworkRequest };
        Self {
            api_base_url_type,
            api_endpoint_url_type,
            crate_ident,
            endpoint_ident: format_ident!("{}Endpoint", parsed_enum.enum_ident),
            // TODO: We use `2` suffix here to prevent the name clash with the current
            // implementation
            request_builder_ident: format_ident!("{}Builder2", parsed_enum.enum_ident),
            request_builder_type,
            request_executor_ident: format_ident!("{}Executor", parsed_enum.enum_ident),
            request_executor_type,
            sparse_field_type: parsed_enum.sparse_field_attr.clone(),
            wp_api_error_type,
            wp_network_request_type,
        }
    }
}
