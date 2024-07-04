use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::Ident;

use super::{ContextAndFilterHandler, PartOf, WpContext};
use crate::{
    parse::{ParsedEnum, RequestType},
    sparse_field_attr::{NamespaceAttr, SparseFieldAttr},
    variant_attr::{ParamsType, UrlPart},
};

const SPARSE_IDENT_PREFIX: &str = "Sparse";

pub fn endpoint_ident(parsed_enum: &ParsedEnum) -> Ident {
    format_ident!("{}Endpoint", parsed_enum.enum_ident)
}

pub fn request_builder_ident(parsed_enum: &ParsedEnum) -> Ident {
    format_ident!("{}Builder", parsed_enum.enum_ident)
}

pub fn api_base_url_type(crate_ident: &Ident) -> TokenStream {
    quote! { std::sync::Arc<#crate_ident::request::endpoint::ApiBaseUrl> }
}

pub fn request_builder_type(crate_ident: &Ident) -> TokenStream {
    quote! { std::sync::Arc<#crate_ident::request::RequestBuilder> }
}

pub fn output_type(
    output_token_tree: Vec<TokenTree>,
    context_and_filter_handler: ContextAndFilterHandler,
) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterNoContext => TokenStream::from_iter(output_token_tree),
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(context) => output_token_tree
            .into_iter()
            .map(|token| {
                if let TokenTree::Ident(ident) = token {
                    let new_ident = if let Some(ident_without_sparse_prefix) =
                        ident.to_string().strip_prefix(SPARSE_IDENT_PREFIX)
                    {
                        format_ident!(
                            "{}With{}Context",
                            ident_without_sparse_prefix,
                            context.to_string()
                        )
                    } else {
                        ident
                    };
                    quote! { #new_ident }
                } else {
                    quote! { #token }
                }
            })
            .collect::<TokenStream>(),
    }
}

pub fn fn_signature(
    part_of: PartOf,
    variant_ident: &Ident,
    url_parts: &[UrlPart],
    params_type: &ParamsType,
    request_type: RequestType,
    context_and_filter_handler: ContextAndFilterHandler,
    sparse_field_type: &SparseFieldAttr,
) -> TokenStream {
    let fn_name = fn_name(variant_ident, context_and_filter_handler);
    let url_params = fn_url_params(url_parts);
    let context_param = fn_context_param(context_and_filter_handler);
    let provided_param = fn_provided_param(part_of, params_type, request_type);
    let fields_param = fn_fields_param(context_and_filter_handler, sparse_field_type);
    quote! { fn #fn_name(&self, #url_params #context_param #provided_param #fields_param) }
}

pub fn fn_url_params(url_parts: &[UrlPart]) -> TokenStream {
    let params = url_parts.iter().filter_map(|p| {
        if let UrlPart::Dynamic(p) = p {
            let p_ident = format_ident!("{}", p);
            let p_upper_camel_case = format_ident!("{}", p.to_case(Case::UpperCamel));
            Some(quote! { #p_ident: &#p_upper_camel_case })
        } else {
            None
        }
    });
    quote! { #(#params,)* }
}

pub fn fn_provided_param(
    part_of: PartOf,
    params_type: &ParamsType,
    request_type: RequestType,
) -> TokenStream {
    if let Some(params_type) = params_type.tokens() {
        let tokens = {
            let params_type_token_stream = TokenStream::from_iter(params_type.clone());
            quote! { params: #params_type_token_stream, }
        };
        match part_of {
            // Endpoints don't need the params type if it's a Post request because params will
            // be part of the body.
            PartOf::Endpoint => match request_type {
                crate::parse::RequestType::ContextualGet | crate::parse::RequestType::Delete => {
                    tokens
                }
                crate::parse::RequestType::Post => TokenStream::new(),
            },
            PartOf::RequestBuilder | PartOf::RequestExecutor => tokens,
        }
    } else {
        TokenStream::new()
    }
}

pub fn fn_context_param(context_and_filter_handler: ContextAndFilterHandler) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_)
        | ContextAndFilterHandler::FilterNoContext => TokenStream::new(),
        ContextAndFilterHandler::NoFilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterTakeContextAsArgument => {
            quote! { context: crate::WpContext, }
        }
    }
}

pub fn fn_fields_param(
    context_and_filter_handler: ContextAndFilterHandler,
    sparse_field_type: &SparseFieldAttr,
) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsArgument
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_) => TokenStream::new(),
        ContextAndFilterHandler::FilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterNoContext => {
            let sparse_field_type: &TokenStream = &sparse_field_type.tokens;
            quote! { fields: &[#sparse_field_type] }
        }
    }
}

pub fn fn_name(
    variant_ident: &Ident,
    context_and_filter_handler: ContextAndFilterHandler,
) -> Ident {
    let basic_fn_name = format_ident!("{}", variant_ident.to_string().to_case(Case::Snake));
    match context_and_filter_handler {
        ContextAndFilterHandler::None | ContextAndFilterHandler::NoFilterTakeContextAsArgument => {
            basic_fn_name
        }
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(context) => format_ident!(
            "{}_with_{}_context",
            basic_fn_name,
            context.to_string().to_lowercase()
        ),
        ContextAndFilterHandler::FilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterNoContext => {
            format_ident!("filter_{}", basic_fn_name)
        }
    }
}

fn fn_arg_url_parts(url_parts: &[UrlPart]) -> TokenStream {
    url_parts
        .iter()
        .filter_map(|url_part| match url_part {
            UrlPart::Dynamic(dynamic_part) => {
                let d = format_ident!("{}", dynamic_part);
                Some(quote! { #d, })
            }
            UrlPart::Static(_) => None,
        })
        .collect::<TokenStream>()
}

fn fn_arg_context(context_and_filter_handler: ContextAndFilterHandler) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::FilterNoContext
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_) => TokenStream::new(),
        ContextAndFilterHandler::NoFilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterTakeContextAsArgument => {
            quote! { context, }
        }
    }
}

fn fn_arg_provided_params(
    part_of: PartOf,
    params_type: &ParamsType,
    request_type: RequestType,
) -> TokenStream {
    if params_type.tokens().is_some() {
        let tokens = quote! { params, };
        match part_of {
            // Endpoints don't need the params type if it's a Post request because params will
            // be part of the body.
            PartOf::Endpoint => match request_type {
                crate::parse::RequestType::ContextualGet | crate::parse::RequestType::Delete => {
                    tokens
                }
                crate::parse::RequestType::Post => TokenStream::new(),
            },
            PartOf::RequestBuilder | PartOf::RequestExecutor => tokens,
        }
    } else {
        TokenStream::new()
    }
}

fn fn_arg_fields(context_and_filter_handler: ContextAndFilterHandler) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsArgument
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_) => TokenStream::new(),
        ContextAndFilterHandler::FilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterNoContext => quote! { fields, },
    }
}

pub fn fn_body_get_url_from_api_base_url(
    namespace_attr: &NamespaceAttr,
    url_parts: &[UrlPart],
) -> TokenStream {
    let url_parts = url_parts
        .iter()
        .map(|part| match part {
            UrlPart::Dynamic(dynamic_part) => {
                let ident = format_ident!("{}", dynamic_part);
                quote! { &#ident.to_string() }
            }
            UrlPart::Static(static_part) => quote! { #static_part },
        })
        .collect::<Vec<TokenStream>>();
    let namespace = namespace_attr.token.clone();
    quote! {
        let mut url = self.api_base_url.by_extending_and_splitting_by_forward_slash([ #namespace, #(#url_parts,)* ]);
    }
}

pub fn fn_body_get_url_from_endpoint(
    variant_ident: &Ident,
    url_parts: &[UrlPart],
    params_type: &ParamsType,
    request_type: RequestType,
    context_and_filter_handler: ContextAndFilterHandler,
) -> TokenStream {
    let fn_name = fn_name(variant_ident, context_and_filter_handler);
    let fn_arg_url_parts = fn_arg_url_parts(url_parts);
    let fn_arg_context = fn_arg_context(context_and_filter_handler);
    let fn_arg_provided_params =
        fn_arg_provided_params(PartOf::Endpoint, params_type, request_type);
    let fn_arg_fields = fn_arg_fields(context_and_filter_handler);

    quote! {
        let url = self.endpoint.#fn_name(#fn_arg_url_parts #fn_arg_context #fn_arg_provided_params #fn_arg_fields);
    }
}

pub fn fn_body_query_pairs(params_type: &ParamsType, request_type: RequestType) -> TokenStream {
    match request_type {
        RequestType::ContextualGet | RequestType::Delete => {
            if let Some(tokens) = params_type.tokens() {
                let is_option = if let Some(TokenTree::Ident(ref ident)) = tokens.first() {
                    // TODO: This won't work with `std::option::Option` or `core::option::Option`
                    *ident == "Option"
                } else {
                    false
                };
                if is_option {
                    quote! {
                        if let Some(params) = params {
                            url.query_pairs_mut().extend_pairs(params.query_pairs());
                        }
                    }
                } else {
                    quote! { url.query_pairs_mut().extend_pairs(params.query_pairs()); }
                }
            } else {
                TokenStream::new()
            }
        }
        RequestType::Post => TokenStream::new(),
    }
}

pub fn fn_body_fields_query_pairs(
    crate_ident: &Ident,
    context_and_filter_handler: ContextAndFilterHandler,
) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsArgument
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_) => TokenStream::new(),
        ContextAndFilterHandler::FilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterNoContext => quote! {
            use #crate_ident::SparseField;
            url.query_pairs_mut().append_pair(
                "_fields",
                fields
                    .iter()
                    .map(|f| f.as_str())
                    .collect::<Vec<&str>>()
                    .join(",")
                    .as_str(),
            );
        },
    }
}

pub fn fn_body_context_query_pairs(
    crate_ident: &Ident,
    context_and_filter_handler: ContextAndFilterHandler,
) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None | ContextAndFilterHandler::FilterNoContext => {
            TokenStream::new()
        }
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(context) => {
            let context = format_ident!("{}", context.to_string());
            quote! {
                url.query_pairs_mut().append_pair("context", #crate_ident::WpContext::#context.as_str());
            }
        }
        ContextAndFilterHandler::NoFilterTakeContextAsArgument
        | ContextAndFilterHandler::FilterTakeContextAsArgument => quote! {
            url.query_pairs_mut().append_pair("context", context.as_str());
        },
    }
}

pub fn fn_body_build_request_from_url(
    params_type: &ParamsType,
    request_type: RequestType,
) -> TokenStream {
    match request_type {
        RequestType::ContextualGet => quote! {
            self.inner.get(url)
        },
        RequestType::Delete => quote! {
            self.inner.delete(url)
        },
        RequestType::Post => {
            if params_type.tokens().is_some() {
                quote! {
                    self.inner.post(url, params)
                }
            } else {
                quote! {
                    self.inner.post(url)
                }
            }
        }
    }
}

pub fn fn_body_get_request_from_request_builder(
    variant_ident: &Ident,
    url_parts: &[UrlPart],
    params_type: &ParamsType,
    request_type: RequestType,
    context_and_filter_handler: ContextAndFilterHandler,
) -> TokenStream {
    let fn_name = fn_name(variant_ident, context_and_filter_handler);
    let fn_arg_url_parts = fn_arg_url_parts(url_parts);
    let fn_arg_context = fn_arg_context(context_and_filter_handler);
    let fn_arg_provided_params =
        fn_arg_provided_params(PartOf::RequestExecutor, params_type, request_type);
    let fn_arg_fields = fn_arg_fields(context_and_filter_handler);

    quote! {
        let request = self.request_builder.#fn_name(#fn_arg_url_parts #fn_arg_context #fn_arg_provided_params #fn_arg_fields);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::too_many_arguments)]
    use std::str::FromStr;

    use crate::sparse_field_attr;

    use super::*;
    use proc_macro2::Literal;
    use quote::ToTokens;
    use rstest::rstest;
    use syn::parse_quote;

    #[rstest]
    #[case(&[UrlPart::Static("users".to_string())], "")]
    #[case(&[UrlPart::Dynamic("user_id".to_string())], "user_id : & UserId ,")]
    #[case(&[UrlPart::Static("users".to_string()), UrlPart::Dynamic("user_id".to_string())], "user_id : & UserId ,")]
    #[case(&[UrlPart::Dynamic("user_id".to_string()), UrlPart::Dynamic("user_type".to_string())], "user_id : & UserId , user_type : & UserType ,")]
    fn test_fn_url_params(#[case] url_parts: &[UrlPart], #[case] expected_str: &str) {
        assert_eq!(fn_url_params(url_parts).to_string(), expected_str);
    }

    #[rstest]
    #[case(PartOf::Endpoint, &ParamsType::new(None), RequestType::ContextualGet, "")]
    #[case(PartOf::Endpoint, &ParamsType::new(Some(vec![])), RequestType::ContextualGet, "")]
    #[case(
        PartOf::Endpoint,
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ""
    )]
    #[case(
        PartOf::RequestBuilder,
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        "params : &UserCreateParams ,"
    )]
    #[case(
        PartOf::RequestExecutor,
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        "params : &UserCreateParams ,"
    )]
    #[case(
        PartOf::Endpoint,
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "params : &UserListParams ,"
    )]
    #[case(
        PartOf::Endpoint,
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "params : &UserListParams ,"
    )]
    fn test_fn_provided_param(
        #[case] part_of: PartOf,
        #[case] params_type: &ParamsType,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_provided_param(part_of, params_type, request_type).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, "")]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        ""
    )]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsArgument,
        "context : crate :: WpContext ,"
    )]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        ""
    )]
    #[case(
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "context : crate :: WpContext ,"
    )]
    fn test_fn_context_param(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_context_param(context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case("List", ContextAndFilterHandler::None, "list")]
    #[case(
        "List",
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "list_with_edit_context"
    )]
    #[case("List", ContextAndFilterHandler::NoFilterTakeContextAsArgument, "list")]
    #[case(
        "ListContents",
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "list_contents_with_embed_context"
    )]
    #[case(
        "List",
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "filter_list"
    )]
    fn test_fn_name(
        #[case] ident: &str,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_name(&format_ident!("{}", ident), context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, quote! { SparseUserField }, "")]
    #[case(ContextAndFilterHandler::NoFilterTakeContextAsArgument, quote! { SparseUserField }, "")]
    #[case(ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::View), quote! { SparseUserField }, "")]
    #[case(
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        quote! { SparseUserField },
        "fields : & [SparseUserField]"
    )]
    #[case(
        ContextAndFilterHandler::FilterNoContext,
        quote! { crate::SparseUserField },
        "fields : & [crate :: SparseUserField]"
    )]
    fn test_fn_fields_param(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        // Don't use the `sparse_field_type` fixture so we can test multi segment sparse field type
        #[case] sparse_field_type: TokenStream,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_fields_param(
                context_and_filter_handler,
                &SparseFieldAttr {
                    tokens: sparse_field_type,
                }
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(url_static_users(), "")]
    #[case(url_users_with_user_id(), "user_id ,")]
    #[case(vec![UrlPart::Static("users".into()), UrlPart::Dynamic("user_id".into()), UrlPart::Dynamic("user_type".into())], "user_id , user_type ,")]
    fn test_fn_arg_url_parts(#[case] url_parts: Vec<UrlPart>, #[case] expected_str: &str) {
        assert_eq!(fn_arg_url_parts(&url_parts).to_string(), expected_str);
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, "")]
    #[case(ContextAndFilterHandler::NoFilterTakeContextAsArgument, "context ,")]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        ""
    )]
    #[case(ContextAndFilterHandler::FilterTakeContextAsArgument, "context ,")]
    #[case(ContextAndFilterHandler::FilterNoContext, "")]
    fn test_fn_arg_context(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_arg_context(context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(PartOf::Endpoint, &ParamsType::new(None), RequestType::ContextualGet, "")]
    #[case(PartOf::Endpoint, &referenced_params_type("UserCreateParams"), RequestType::Post, "")]
    #[case(PartOf::RequestBuilder, &referenced_params_type("UserCreateParams"), RequestType::Post, "params ,")]
    #[case(PartOf::RequestExecutor, &referenced_params_type("UserCreateParams"), RequestType::Post, "params ,")]
    #[case(PartOf::Endpoint, &referenced_params_type("UserListParams"), RequestType::ContextualGet, "params ,")]
    #[case(PartOf::Endpoint, &referenced_params_type("UserListParams"), RequestType::ContextualGet, "params ,")]
    #[case(PartOf::Endpoint, &referenced_params_type("UserListParams"), RequestType::Delete, "params ,")]
    #[case(PartOf::Endpoint, &referenced_params_type("UserListParams"), RequestType::Post, "")]
    fn test_fn_arg_provided_params(
        #[case] part_of: PartOf,
        #[case] params_type: &ParamsType,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_arg_provided_params(part_of, params_type, request_type).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, "")]
    #[case(ContextAndFilterHandler::NoFilterTakeContextAsArgument, "")]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        ""
    )]
    #[case(ContextAndFilterHandler::FilterTakeContextAsArgument, "fields ,")]
    #[case(ContextAndFilterHandler::FilterNoContext, "fields ,")]
    fn test_fn_arg_fields(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_arg_fields(context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(parse_quote!(crate::SparseUser), ContextAndFilterHandler::None, "crate :: SparseUser")]
    #[case(parse_quote!(crate::SparseUser), ContextAndFilterHandler::NoFilterTakeContextAsArgument, "crate :: SparseUser")]
    #[case(
        parse_quote!(crate::SparseUser),
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "crate :: UserWithEditContext"
    )]
    #[case(
        parse_quote!(SparseUser),
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "UserWithEmbedContext"
    )]
    #[case(
        parse_quote!(std::vec::Vec<crate::SparseUser>),
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::View),
        "std :: vec :: Vec < crate :: UserWithViewContext >"
    )]
    #[case(parse_quote!(SparseUser), ContextAndFilterHandler::FilterTakeContextAsArgument, "SparseUser")]
    #[case(parse_quote!(Vec<SparseUser>), ContextAndFilterHandler::FilterNoContext, "Vec < SparseUser >")]
    fn test_output_type(
        #[case] output_token_stream: TokenStream,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            output_type(
                output_token_stream.into_iter().collect(),
                context_and_filter_handler
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(
        PartOf::Endpoint,
        format_ident!("Create"),
        url_static_users(),
        &ParamsType::new(None),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn create (& self ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Create"),
        url_static_users(),
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "fn filter_create (& self , fields : & [SparseUserField])")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("Create"),
        url_static_users(),
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "fn filter_create (& self , params : &UserCreateParams , fields : & [SparseUserField])")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("Create"),
        url_static_users(),
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "fn filter_create (& self , params : &UserCreateParams , fields : & [SparseUserField])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Delete"),
        url_users_with_user_id(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "fn delete (& self , user_id : & UserId , params : &UserDeleteParams ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Delete"),
        url_users_with_user_id(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::FilterNoContext,
        "fn filter_delete (& self , user_id : & UserId , params : &UserDeleteParams , fields : & [SparseUserField])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("DeleteMe"),
        url_static_users(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "fn delete_me (& self , params : &UserDeleteParams ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsArgument,
        "fn list (& self , context : crate :: WpContext , params : &UserListParams ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "fn filter_list (& self , context : crate :: WpContext , params : &UserListParams , fields : & [SparseUserField])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        &ParamsType::new(None),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "fn retrieve_with_embed_context (& self , user_id : & UserId ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        &ParamsType::new(None),
        RequestType::ContextualGet,
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "fn filter_retrieve (& self , user_id : & UserId , context : crate :: WpContext , fields : & [SparseUserField])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn update (& self , user_id : & UserId ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "fn filter_update (& self , user_id : & UserId , fields : & [SparseUserField])")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn update (& self , user_id : & UserId , params : &UserUpdateParams ,)")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn update (& self , user_id : & UserId , params : &UserUpdateParams ,)")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "fn filter_update (& self , user_id : & UserId , params : &UserUpdateParams , fields : & [SparseUserField])")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "fn filter_update (& self , user_id : & UserId , params : &UserUpdateParams , fields : & [SparseUserField])")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "fn list_with_edit_context (& self , params : &UserListParams ,)")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "fn list_with_edit_context (& self , params : &UserListParams ,)")]
    fn test_fn_signature(
        #[case] part_of: PartOf,
        #[case] variant_ident: Ident,
        #[case] url_parts: Vec<UrlPart>,
        #[case] params_type: &ParamsType,
        #[case] request_type: RequestType,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
        sparse_field_type: SparseFieldAttr,
    ) {
        assert_eq!(
            fn_signature(
                part_of,
                &variant_ident,
                &url_parts,
                params_type,
                request_type,
                context_and_filter_handler,
                &sparse_field_type,
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . create () ;")]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "let url = self . endpoint . filter_create (fields ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . delete (user_id , params ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::FilterNoContext,
        "let url = self . endpoint . filter_delete (user_id , params , fields ,) ;")]
    #[case(
        format_ident!("DeleteMe"),
        url_static_users(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . delete_me (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "let url = self . endpoint . list_with_edit_context (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "let url = self . endpoint . filter_list (context , params , fields ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        &ParamsType::new(None),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "let url = self . endpoint . retrieve_with_embed_context (user_id ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        &ParamsType::new(None),
        RequestType::ContextualGet,
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "let url = self . endpoint . filter_retrieve (user_id , context , fields ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . update (user_id ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "let url = self . endpoint . filter_update (user_id , fields ,) ;")]
    fn test_fn_body_get_url_from_endpoint(
        #[case] variant_ident: Ident,
        #[case] url_parts: Vec<UrlPart>,
        #[case] params_type: &ParamsType,
        #[case] request_type: RequestType,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_get_url_from_endpoint(
                &variant_ident,
                &url_parts,
                params_type,
                request_type,
                context_and_filter_handler
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(
        url_static_users(),
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash ([\"/wp/v2\" , \"users\" ,]) ;"
    )]
    #[case(
        url_users_with_user_id(),
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash ([\"/wp/v2\" , \"users\" , & user_id . to_string () ,]) ;"
    )]
    #[case(
        url_users_with_user_id(),
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash ([\"/wp/v2\" , \"users\" , & user_id . to_string () ,]) ;"
    )]
    #[case(
        vec![UrlPart::Dynamic("user_id".to_string()), UrlPart::Dynamic("user_type".to_string())],
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash ([\"/wp/v2\" , & user_id . to_string () , & user_type . to_string () ,]) ;"
    )]
    #[case(
        vec![UrlPart::Static("users".to_string()), UrlPart::Dynamic("user_id".to_string()), UrlPart::Dynamic("user_type".to_string()), ],
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash ([\"/wp/v2\" , \"users\" , & user_id . to_string () , & user_type . to_string () ,]) ;"
    )]
    #[case(
        vec![UrlPart::Static("users".to_string()), UrlPart::Static("me".to_string()), UrlPart::Dynamic("user_id".to_string()), UrlPart::Dynamic("user_type".to_string()), ],
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash ([\"/wp/v2\" , \"users\" , \"me\" , & user_id . to_string () , & user_type . to_string () ,]) ;"
    )]
    fn test_fn_body_get_url_from_api_base_url(
        #[case] url_parts: Vec<UrlPart>,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_get_url_from_api_base_url(
                &NamespaceAttr {
                    token: quote! { "/wp/v2" }.into_iter().next().unwrap(),
                },
                &url_parts
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(&ParamsType::new(None), RequestType::ContextualGet, "")]
    #[case(
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "url . query_pairs_mut () . extend_pairs (params . query_pairs ()) ;"
    )]
    #[case(
        &option_referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "if let Some (params) = params { url . query_pairs_mut () . extend_pairs (params . query_pairs ()) ; }"
    )]
    #[case(
        &option_referenced_params_type("UserListParams"),
        RequestType::Post,
        ""
    )]
    fn test_fn_body_query_pairs(
        #[case] params: &ParamsType,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_query_pairs(params, request_type).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, true)]
    #[case(ContextAndFilterHandler::NoFilterTakeContextAsArgument, true)]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        true
    )]
    #[case(ContextAndFilterHandler::FilterTakeContextAsArgument, false)]
    #[case(ContextAndFilterHandler::FilterNoContext, false)]
    fn test_fn_body_fields_query_pairs(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] is_empty: bool,
    ) {
        // Test if the `_fields` query pair is included or not
        // Since this query pair is static, there is no need to compare the string value
        let crate_ident = format_ident!("crate");
        assert_eq!(
            fn_body_fields_query_pairs(&crate_ident, context_and_filter_handler).is_empty(),
            is_empty
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, "")]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsArgument,
        "url . query_pairs_mut () . append_pair (\"context\" , context . as_str ()) ;"
    )]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "url . query_pairs_mut () . append_pair (\"context\" , crate :: WpContext :: Edit . as_str ()) ;"
    )]
    #[case(
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "url . query_pairs_mut () . append_pair (\"context\" , context . as_str ()) ;"
    )]
    #[case(ContextAndFilterHandler::FilterNoContext, "")]
    fn test_fn_body_context_query_pairs(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        let crate_ident = format_ident!("crate");
        assert_eq!(
            fn_body_context_query_pairs(&crate_ident, context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(&ParamsType::new(None), RequestType::ContextualGet, "self . inner . get (url)")]
    #[case(
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "self . inner . get (url)"
    )]
    #[case(&ParamsType::new(None), RequestType::Delete, "self . inner . delete (url)")]
    #[case(
        &referenced_params_type("UserListParams"),
        RequestType::Delete,
        "self . inner . delete (url)"
    )]
    #[case(&ParamsType::new(None), RequestType::Post, "self . inner . post (url)")]
    #[case(
        &referenced_params_type("UserListParams"),
        RequestType::Post,
        "self . inner . post (url , params)"
    )]
    fn test_fn_body_build_request_from_url(
        #[case] params: &ParamsType,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_build_request_from_url(params, request_type).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . create (params ,) ;")]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        &referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "let request = self . request_builder . filter_create (params , fields ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . delete (user_id , params ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::FilterNoContext,
        "let request = self . request_builder . filter_delete (user_id , params , fields ,) ;")]
    #[case(
        format_ident!("DeleteMe"),
        url_static_users(),
        &referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . delete_me (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "let request = self . request_builder . list_with_edit_context (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        &referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "let request = self . request_builder . filter_list (context , params , fields ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        &ParamsType::new(None),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "let request = self . request_builder . retrieve_with_embed_context (user_id ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        &ParamsType::new(None),
        RequestType::ContextualGet,
        ContextAndFilterHandler::FilterTakeContextAsArgument,
        "let request = self . request_builder . filter_retrieve (user_id , context , fields ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . update (user_id , params ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        &referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::FilterNoContext,
        "let request = self . request_builder . filter_update (user_id , params , fields ,) ;")]
    fn test_fn_body_get_request_from_request_builder(
        #[case] variant_ident: Ident,
        #[case] url_parts: Vec<UrlPart>,
        #[case] params_type: &ParamsType,
        #[case] request_type: RequestType,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_get_request_from_request_builder(
                &variant_ident,
                &url_parts,
                params_type,
                request_type,
                context_and_filter_handler
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest::fixture]
    fn sparse_field_type() -> SparseFieldAttr {
        SparseFieldAttr {
            tokens: quote! { SparseUserField },
        }
    }

    fn referenced_params_type(str: &str) -> ParamsType {
        ParamsType::new(Some(vec![
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(
                '&',
                proc_macro2::Spacing::Joint,
            )),
            format_ident!("{}", str).into(),
        ]))
    }

    fn option_referenced_params_type(str: &str) -> ParamsType {
        ParamsType::new(Some(vec![
            format_ident!("Option").into(),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(
                '<',
                proc_macro2::Spacing::Joint,
            )),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(
                '&',
                proc_macro2::Spacing::Joint,
            )),
            format_ident!("{}", str).into(),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(
                '>',
                proc_macro2::Spacing::Joint,
            )),
        ]))
    }

    fn url_static_users() -> Vec<UrlPart> {
        vec![UrlPart::Static("users".into())]
    }

    fn url_users_with_user_id() -> Vec<UrlPart> {
        vec![
            UrlPart::Static("users".into()),
            UrlPart::Dynamic("user_id".into()),
        ]
    }
}
