use std::fmt::Display;

use helpers_to_generate_tokens::*;
use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use syn::Ident;

use crate::{
    outer_attr::{NamespaceAttr, SparseFieldAttr},
    parse::{ParsedEnum, ParsedVariant, RequestType},
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
    let static_api_base_url_type = &config.static_types.api_base_url;
    let static_wp_authentication_type = &config.static_types.wp_authentication;
    let static_inner_request_builder_type = &config.static_types.inner_request_builder;
    let static_request_executor_type = &config.static_types.request_executor;
    let static_wp_api_error_type = &config.static_types.wp_api_error;
    let generated_request_builder_ident = &config.generated_idents.request_builder;
    let generated_request_executor_ident = &config.generated_idents.request_executor;

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
                    pub async #fn_signature -> Result<#output_type, #static_wp_api_error_type> {
                        #request_from_request_builder
                        self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
                   }
                }
            })
            .collect::<TokenStream>()
    });

    quote! {
        #[derive(Debug, uniffi::Object)]
        pub struct #generated_request_executor_ident {
            request_builder: #generated_request_builder_ident,
            request_executor: #static_request_executor_type,
        }
        impl #generated_request_executor_ident {
            pub fn new(api_base_url: #static_api_base_url_type, authentication: #static_wp_authentication_type, request_executor: #static_request_executor_type) -> Self {
                Self {
                    request_builder: #generated_request_builder_ident::new(api_base_url, authentication),
                    request_executor,
                }
            }
        }
        #[uniffi::export]
        impl #generated_request_executor_ident {
            #(#functions)*
        }
    }
}

fn generate_request_builder(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let static_api_base_url_type = &config.static_types.api_base_url;
    let static_inner_request_builder_type = &config.static_types.inner_request_builder;
    let static_wp_authentication_type = &config.static_types.wp_authentication;
    let static_wp_network_request_type = &config.static_types.wp_network_request;
    let generated_endpoint_ident = &config.generated_idents.endpoint;
    let generated_request_builder_ident = &config.generated_idents.request_builder;

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
                    pub #fn_signature -> #static_wp_network_request_type {
                        #url_from_endpoint
                        #fn_body_build_request_from_url
                    }
                }
            })
            .collect::<TokenStream>()
    });

    quote! {
        #[derive(Debug, uniffi::Object)]
        pub struct #generated_request_builder_ident {
            endpoint: #generated_endpoint_ident,
            inner: #static_inner_request_builder_type,
        }
        impl #generated_request_builder_ident {
            pub fn new(api_base_url: #static_api_base_url_type, authentication: #static_wp_authentication_type) -> Self {
                Self {
                    endpoint: #generated_endpoint_ident::new(api_base_url),
                    inner: #static_inner_request_builder_type::new(authentication),
                }
            }
        }
        #[uniffi::export]
        impl #generated_request_builder_ident {
            #(#functions)*
        }
    }
}

fn generate_endpoint_type(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let static_api_base_url_type = &config.static_types.api_base_url;
    let static_api_endpoint_url_type = &config.static_types.api_endpoint_url;
    let generated_endpoint_ident = &config.generated_idents.endpoint;

    let functions = parsed_enum.variants.iter().map(|variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;
        let request_type = variant.attr.request_type;
        let url_from_api_base_url =
            fn_body_get_url_from_api_base_url(&config.namespace_attr, url_parts);
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
                let fields_query_pairs =
                    fn_body_fields_query_pairs(&config.crate_ident, context_and_filter_handler);
                quote! {
                    pub #fn_signature -> #static_api_endpoint_url_type {
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
        pub struct #generated_endpoint_ident {
            api_base_url: #static_api_base_url_type,
        }

        impl #generated_endpoint_ident {
            pub fn new(api_base_url: #static_api_base_url_type) -> Self {
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
    NoFilterTakeContextAsFunctionName(WpContext),
    FilterTakeContextAsArgument,
    FilterNoContext,
}

impl ContextAndFilterHandler {
    fn from_request_type(request_type: RequestType) -> Vec<Self> {
        match request_type {
            crate::parse::RequestType::Get => {
                vec![Self::None, Self::FilterNoContext]
            }
            crate::parse::RequestType::ContextualGet => {
                let mut v: Vec<Self> = WpContext::iter()
                    .map(Self::NoFilterTakeContextAsFunctionName)
                    .collect();
                v.push(Self::FilterTakeContextAsArgument);
                v
            }
            crate::parse::RequestType::Delete | crate::parse::RequestType::Post => {
                vec![Self::None]
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
    pub sparse_field_type: SparseFieldAttr,
    pub namespace_attr: NamespaceAttr,
    pub generated_idents: ConfigGeneratedIdents,
    pub static_types: ConfigStaticTypes,
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
        let generated_idents = ConfigGeneratedIdents::new(parsed_enum);
        let static_types = ConfigStaticTypes::new(&crate_ident);

        Self {
            crate_ident,
            sparse_field_type: parsed_enum.outer_attr.sparse_field_attr.clone(),
            namespace_attr: parsed_enum.outer_attr.namespace_attr.clone(),
            generated_idents,
            static_types,
        }
    }
}

#[derive(Debug)]
pub struct ConfigStaticTypes {
    pub api_base_url: TokenStream,
    pub api_endpoint_url: TokenStream,
    pub inner_request_builder: TokenStream,
    pub request_executor: TokenStream,
    pub wp_api_error: TokenStream,
    pub wp_authentication: TokenStream,
    pub wp_network_request: TokenStream,
}

impl ConfigStaticTypes {
    fn new(crate_ident: &Ident) -> Self {
        Self {
            api_base_url: quote! { std::sync::Arc<#crate_ident::request::endpoint::ApiBaseUrl> },
            api_endpoint_url: quote! { #crate_ident::request::endpoint::ApiEndpointUrl },
            inner_request_builder: quote! { #crate_ident::request::InnerRequestBuilder },
            request_executor: quote! { std::sync::Arc<dyn #crate_ident::request::RequestExecutor> },
            wp_api_error: quote! { #crate_ident::WpApiError },
            wp_authentication: quote! { #crate_ident::WpAuthentication },
            wp_network_request: quote! { #crate_ident::request::WpNetworkRequest },
        }
    }
}

#[derive(Debug)]
pub struct ConfigGeneratedIdents {
    pub endpoint: Ident,
    pub request_builder: Ident,
    pub request_executor: Ident,
}

impl ConfigGeneratedIdents {
    fn new(parsed_enum: &ParsedEnum) -> Self {
        Self {
            endpoint: format_ident!("{}Endpoint", parsed_enum.enum_ident),
            request_builder: format_ident!("{}Builder", parsed_enum.enum_ident),
            request_executor: format_ident!("{}Executor", parsed_enum.enum_ident),
        }
    }
}
