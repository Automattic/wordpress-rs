use std::fmt::Display;

use helpers_to_generate_tokens::*;
use proc_macro_crate::FoundCrate;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use serde::{Deserialize, Deserializer, de::Error};
use syn::Ident;

use crate::{
    parse::{ParsedEnum, RequestType},
    variant_attr::FilterByType,
};

mod helpers_to_generate_tokens;

pub(crate) fn generate_types(parsed_enum: &ParsedEnum, crate_config: &CrateConfig) -> TokenStream {
    let config = Config::new(parsed_enum);
    TokenStream::from_iter(
        &mut [
            generate_endpoint_type(&config, parsed_enum),
            generate_request_builder(&config, parsed_enum),
            generate_async_request_executor(&config, parsed_enum, crate_config),
        ]
        .into_iter(),
    )
}

fn generate_async_request_executor(
    config: &Config,
    parsed_enum: &ParsedEnum,
    crate_config: &CrateConfig,
) -> TokenStream {
    let crate_ident = &config.crate_ident;
    let static_api_url_resolver = &config.static_types.api_url_resolver;
    let static_delegate_type = &crate_config.delegate_type;
    let static_delegate_type = quote! { #static_delegate_type };
    let error_type = &crate_config.error_type;
    let generated_request_builder_ident = &config.generated_idents.request_builder;
    let generated_request_executor_ident = &config.generated_idents.request_executor;

    let (exported_functions, unexported_functions) = parsed_enum.variants.iter().fold((vec![], vec![]), |(mut exported, mut unexported), variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;

        (exported, unexported) = ContextAndFilterHandler::from_request_type(
            variant.attr.request_type,
            variant.attr.filter_by.clone(),
            &variant.attr.available_contexts,
        )
        .into_iter()
        .fold((exported, unexported), |(mut exported, mut unexported), context_and_filter_handler| {
            let request_from_request_builder = fn_body_get_request_from_request_builder(
                &variant.variant_ident,
                url_parts,
                params_type.as_ref(),
                variant.attr.request_type,
                &context_and_filter_handler,
            );
            let fn_signature = fn_signature(
                PartOf::RequestExecutor,
                &variant.variant_ident,
                url_parts,
                params_type.as_ref(),
                variant.attr.request_type,
                &context_and_filter_handler,
            );
            let fn_signature_cancellable = append_context_param(fn_signature.clone());
            let fn_signature_body = invoke_cancellation_variant(fn_signature.clone());
            let response_type_ident = ident_response_type(
                &parsed_enum.enum_ident,
                &variant.variant_ident,
                &context_and_filter_handler,
            );

            let uncancellable = quote! {
                pub async #fn_signature -> Result<#response_type_ident, #error_type> {
                    #fn_signature_body
                }
            };
            let cancellable = quote! {
                pub async #fn_signature_cancellable -> Result<#response_type_ident, #error_type> {
                    use #crate_ident::api_error::MaybeWpError;
                    use #crate_ident::middleware::PerformsRequests;
                    use #crate_ident::request::NetworkRequestAccessor;
                    let perform_request = async || {
                        #request_from_request_builder
                        let request_url: String = request.url().into();
                        let response = self.perform(std::sync::Arc::new(request), context.clone()).await?;
                        let response_status_code = response.status_code;
                        let parsed_response = response.parse();
                        let unauthorized = parsed_response.is_unauthorized_error().unwrap_or_default() || (response_status_code == 401 && self.fetch_authentication_state().await.map(|auth_state| auth_state.is_unauthorized()).unwrap_or_default());

                        Ok((parsed_response, unauthorized, request_url))
                    };

                    let mut parsed_response;
                    let mut unauthorized;
                    let mut request_url;

                    // The Rust compiler has trouble figuring out the return type. The statement below helps it.
                    let result: Result<_, #error_type> = perform_request().await;
                    (parsed_response, unauthorized, request_url) = result?;

                    // If the auth provider successfully refreshed the authentication, we can retry the request.
                    if unauthorized && self.delegate.auth_provider.refresh().await {
                        // The response of the retried request will be returned instead of the original one.
                        (parsed_response, unauthorized, request_url) = perform_request().await?;
                    }

                    if unauthorized {
                        self.delegate.app_notifier.requested_with_invalid_authentication(request_url).await;
                    }

                    parsed_response
               }
            };

            if cfg!(feature = "export-uncancellable-endpoints") {
                exported.push(cancellable);
                unexported.push(uncancellable);
            } else {
                unexported.push(cancellable);
                exported.push(uncancellable);
            }

            (exported, unexported)
        });

        (exported, unexported)
    });

    let generated_return_types = parsed_enum.variants.iter().map(|variant| {
        ContextAndFilterHandler::from_request_type(
            variant.attr.request_type,
            variant.attr.filter_by.clone(),
            &variant.attr.available_contexts
        )
        .into_iter()
        .map(|context_and_filter_handler| {
            let output_type = output_type(variant.attr.output.clone(), &context_and_filter_handler);
            let response_type_ident = ident_response_type(
                &parsed_enum.enum_ident,
                &variant.variant_ident,
                &context_and_filter_handler,
            );
            let fn_parse_as_response_type_ident = ident_fn_parse_as_response_type(&response_type_ident);
            let response_params_type = response_params_type(variant.attr.params.as_ref(), variant.attr.request_type);
            let response_pagination_params_fields = response_params_type.as_ref().map(|p| {
                quote! {
                    #[serde(skip)]
                    pub next_page_params: Option<#p>,
                    #[serde(skip)]
                    pub prev_page_params: Option<#p>,
                }
            });
            let from_concrete_response_impl_for_pagination_params = response_params_type.as_ref().map(|_| {
                quote! {
                    next_page_params: value.next_page_params,
                    prev_page_params: value.prev_page_params,
                }
            }).unwrap_or(quote! {
                next_page_params: None,
                prev_page_params: None,
            });
            let from_parsed_response_impl_for_pagination_params = response_params_type.as_ref().map(|_| {
                quote! {
                    next_page_params: value.next_page_params,
                    prev_page_params: value.prev_page_params,
                }
            });
            // Generic type <P> of `ParsedResponse<T, P>` can't be `None`
            let parsed_response_params_type = response_params_type.unwrap_or(quote! { () });
            quote! {
                #[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record)]
                #[serde(transparent)]
                pub struct #response_type_ident {
                    pub data: #output_type,
                    #[serde(skip)]
                    pub header_map: std::sync::Arc<#crate_ident::request::WpNetworkHeaderMap>,
                    #response_pagination_params_fields
                }
                impl From<#response_type_ident> for #crate_ident::request::ParsedResponse<#output_type, #parsed_response_params_type> {
                    fn from(value: #response_type_ident) -> Self {
                        Self {
                            data: value.data,
                            header_map: value.header_map,
                            #from_concrete_response_impl_for_pagination_params
                        }
                    }
                }
                impl From<#crate_ident::request::ParsedResponse<#output_type, #parsed_response_params_type>> for #response_type_ident {
                    fn from(value: #crate_ident::request::ParsedResponse<#output_type, #parsed_response_params_type>) -> Self {
                        Self {
                            data: value.data,
                            header_map: value.header_map,
                            #from_parsed_response_impl_for_pagination_params
                        }
                    }
                }

                fn #fn_parse_as_response_type_ident(response: #crate_ident::request::WpNetworkResponse) -> Result<#response_type_ident, #error_type> {
                    response.parse()
                }
            }
        })
        .collect::<TokenStream>()
    });

    quote! {
        #(#generated_return_types)*

        #[derive(uniffi::Object)]
        pub struct #generated_request_executor_ident {
            api_url_resolver: #static_api_url_resolver,
            request_builder: #generated_request_builder_ident,
            delegate: #static_delegate_type,
        }
        impl #generated_request_executor_ident {
            pub fn new(api_url_resolver: #static_api_url_resolver, delegate: #static_delegate_type) -> Self {
                Self {
                    api_url_resolver: api_url_resolver.clone(),
                    request_builder: #generated_request_builder_ident::new(api_url_resolver, delegate.auth_provider.clone()),
                    delegate,
                }
            }
        }
        impl #crate_ident::api_client::IsWpApiClientDelegate for #generated_request_executor_ident {
            fn get_delegate(&self) -> &#crate_ident::api_client::WpApiClientDelegate {
                &self.delegate
            }
        }
        #[uniffi::export]
        impl #generated_request_executor_ident {
            #(#exported_functions)*

            pub async fn fetch_authentication_state(&self) -> Result<#crate_ident::request::AuthenticationState, #error_type> {
                #crate_ident::request::fetch_authentication_state(self.delegate.request_executor.clone(), self.api_url_resolver.clone(), self.delegate.auth_provider.clone()).await
            }

            pub fn cancel(&self, context: std::sync::Arc<crate::request::RequestContext>) {
                self.delegate.request_executor.cancel(context);
            }
        }

        impl #generated_request_executor_ident {
            #(#unexported_functions)*
        }
    }
}

fn generate_request_builder(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let static_api_url_resolver = &config.static_types.api_url_resolver;
    let static_inner_request_builder_type = &config.static_types.inner_request_builder;
    let static_auth_provider_type = &config.static_types.auth_provider;
    let static_wp_network_request_type = &config.static_types.wp_network_request;
    let generated_endpoint_ident = &config.generated_idents.endpoint;
    let generated_request_builder_ident = &config.generated_idents.request_builder;

    let functions = parsed_enum.variants.iter().map(|variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;

        ContextAndFilterHandler::from_request_type(
            variant.attr.request_type,
            variant.attr.filter_by.clone(),
            &variant.attr.available_contexts,
        )
        .into_iter()
        .map(|context_and_filter_handler| {
            let url_from_endpoint = fn_body_get_url_from_endpoint(
                &variant.variant_ident,
                url_parts,
                params_type.as_ref(),
                variant.attr.request_type,
                &context_and_filter_handler,
            );
            let fn_signature = fn_signature(
                PartOf::RequestBuilder,
                &variant.variant_ident,
                url_parts,
                params_type.as_ref(),
                variant.attr.request_type,
                &context_and_filter_handler,
            );
            let fn_body_build_request_from_url =
                fn_body_build_request_from_url(params_type.as_ref(), variant.attr.request_type);
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
        pub struct #generated_request_builder_ident {
            endpoint: #generated_endpoint_ident,
            inner: #static_inner_request_builder_type,
        }
        impl #generated_request_builder_ident {
            pub fn new(api_url_resolver: #static_api_url_resolver, auth_provider: #static_auth_provider_type) -> Self {
                Self {
                    endpoint: #generated_endpoint_ident::new(api_url_resolver),
                    inner: #static_inner_request_builder_type::new(auth_provider),
                }
            }
        }
        impl #generated_request_builder_ident {
            #(#functions)*
        }
    }
}

fn generate_endpoint_type(config: &Config, parsed_enum: &ParsedEnum) -> TokenStream {
    let static_api_url_resolver = &config.static_types.api_url_resolver;
    let static_api_endpoint_url_type = &config.static_types.api_endpoint_url;
    let generated_endpoint_ident = &config.generated_idents.endpoint;

    let functions = parsed_enum.variants.iter().map(|variant| {
        let url_parts = variant.attr.url_parts.as_slice();
        let params_type = &variant.attr.params;
        let request_type = variant.attr.request_type;
        let url_from_api_url_resolver =
            fn_body_get_url_from_api_url_resolver(&parsed_enum.enum_ident, url_parts);
        let query_pairs =
            fn_body_query_pairs(&config.crate_ident, params_type.as_ref(), request_type);
        let additional_query_pairs =
            fn_body_additional_query_pairs(&parsed_enum.enum_ident, &variant.variant_ident);

        ContextAndFilterHandler::from_request_type(
            request_type,
            variant.attr.filter_by.clone(),
            &variant.attr.available_contexts,
        )
        .into_iter()
        .map(|context_and_filter_handler| {
            let fn_signature = fn_signature(
                PartOf::Endpoint,
                &variant.variant_ident,
                url_parts,
                params_type.as_ref(),
                request_type,
                &context_and_filter_handler,
            );
            let context_query_pair =
                fn_body_context_query_pairs(&config.crate_ident, &context_and_filter_handler);
            let fields_query_pairs =
                fn_body_fields_query_pairs(&config.crate_ident, &context_and_filter_handler);
            quote! {
                pub #fn_signature -> #static_api_endpoint_url_type {
                    #url_from_api_url_resolver
                    #context_query_pair
                    #query_pairs
                    #additional_query_pairs
                    #fields_query_pairs
                    url.into()
                }
            }
        })
        .collect::<TokenStream>()
    });

    quote! {
        pub struct #generated_endpoint_ident {
            api_url_resolver: #static_api_url_resolver,
        }

        impl #generated_endpoint_ident {
            pub fn new(api_url_resolver: #static_api_url_resolver) -> Self {
                Self { api_url_resolver }
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

#[derive(Debug, Clone)]
pub enum ContextAndFilterHandler {
    None,
    NoFilterTakeContextAsFunctionName(WpContext),
    FilterTakeContextAsFunctionName(WpContext, FilterByType),
    FilterNoContext(FilterByType),
}

impl ContextAndFilterHandler {
    fn from_request_type(
        request_type: RequestType,
        filter_by_type: Option<FilterByType>,
        available_contexts: &[WpContext],
    ) -> Vec<Self> {
        match request_type {
            crate::parse::RequestType::Get => {
                let mut v = vec![Self::None];
                if let Some(filter_by_type) = filter_by_type {
                    v.push(Self::FilterNoContext(filter_by_type));
                }
                v
            }
            crate::parse::RequestType::ContextualGet
            | crate::parse::RequestType::ContextualPaged => {
                let mut v = vec![];
                available_contexts.iter().for_each(|context| {
                    v.push(Self::NoFilterTakeContextAsFunctionName(*context));
                    if let Some(ref filter_by_type) = filter_by_type {
                        v.push(Self::FilterTakeContextAsFunctionName(
                            *context,
                            filter_by_type.clone(),
                        ));
                    }
                });
                v
            }
            crate::parse::RequestType::Delete | crate::parse::RequestType::Post => {
                vec![Self::None]
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WpContext {
    Edit,
    Embed,
    View,
}

impl WpContext {
    pub fn all() -> Vec<Self> {
        vec![Self::Edit, Self::Embed, Self::View]
    }
}

impl Display for WpContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WpContext::Edit => "Edit",
            WpContext::Embed => "Embed",
            WpContext::View => "View",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Deserialize)]
pub struct CrateConfig {
    #[serde(deserialize_with = "from_string_to_token_stream")]
    error_type: TokenStream,
    #[serde(deserialize_with = "from_string_to_token_stream")]
    delegate_type: TokenStream,
}

fn from_string_to_token_stream<'de, D>(deserializer: D) -> Result<TokenStream, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(D::Error::custom)
}

#[derive(Debug)]
pub struct Config {
    pub crate_ident: Ident,
    pub generated_idents: ConfigGeneratedIdents,
    pub static_types: ConfigStaticTypes,
}

impl Config {
    fn new(parsed_enum: &ParsedEnum) -> Self {
        let crate_name = "wp_api";
        let found_crate = proc_macro_crate::crate_name(crate_name)
            .unwrap_or_else(|_| panic!("{crate_name} is not present in `Cargo.toml`"));

        let crate_ident = match found_crate {
            FoundCrate::Itself => format_ident!("crate"),
            FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        };
        let generated_idents = ConfigGeneratedIdents::new(parsed_enum);
        let static_types = ConfigStaticTypes::new(&crate_ident);

        Self {
            crate_ident,
            generated_idents,
            static_types,
        }
    }
}

#[derive(Debug)]
pub struct ConfigStaticTypes {
    pub api_url_resolver: TokenStream,
    pub api_endpoint_url: TokenStream,
    pub inner_request_builder: TokenStream,
    pub auth_provider: TokenStream,
    pub wp_network_request: TokenStream,
}

impl ConfigStaticTypes {
    fn new(crate_ident: &Ident) -> Self {
        Self {
            api_url_resolver: quote! { std::sync::Arc<dyn #crate_ident::request::endpoint::ApiUrlResolver> },
            api_endpoint_url: quote! { #crate_ident::request::endpoint::ApiEndpointUrl },
            inner_request_builder: quote! { #crate_ident::request::InnerRequestBuilder },
            auth_provider: quote! { std::sync::Arc<#crate_ident::auth::WpAuthenticationProvider> },
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
