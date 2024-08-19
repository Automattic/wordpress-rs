use convert_case::{Case, Casing};
use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::Ident;

use super::{ContextAndFilterHandler, PartOf, WpContext};
use crate::{
    parse::RequestType,
    variant_attr::{ParamsType, UrlPart},
};

const SPARSE_IDENT_PREFIX: &str = "Sparse";

pub fn output_type(
    output_token_tree: Vec<TokenTree>,
    context_and_filter_handler: &ContextAndFilterHandler,
) -> TokenStream {
    let strip_sparse_prefix = |token_tree: Vec<TokenTree>, context: Option<WpContext>| {
        token_tree
            .into_iter()
            .map(|token| {
                if let TokenTree::Ident(ident) = token {
                    let new_ident = if let Some(ident_without_sparse_prefix) =
                        ident.to_string().strip_prefix(SPARSE_IDENT_PREFIX)
                    {
                        if let Some(context) = context {
                            // For example, given `SparseFoo`, it may be `FooWithEditContext`
                            format_ident!(
                                "{}With{}Context",
                                ident_without_sparse_prefix,
                                context.to_string()
                            )
                        } else {
                            // For example, given `SparseFoo`, it may be `Foo`
                            format_ident!("{}", ident_without_sparse_prefix,)
                        }
                    } else {
                        ident
                    };
                    quote! { #new_ident }
                } else {
                    quote! { #token }
                }
            })
            .collect::<TokenStream>()
    };
    match context_and_filter_handler {
        ContextAndFilterHandler::None => strip_sparse_prefix(output_token_tree, None),
        ContextAndFilterHandler::FilterTakeContextAsFunctionName(context, _) => output_token_tree
            .into_iter()
            .map(|token| {
                if let TokenTree::Ident(ident) = token {
                    let new_ident = if ident.to_string().starts_with(SPARSE_IDENT_PREFIX) {
                        // For example, given `SparseFoo`, it may be `SparseFooWithEditContext`
                        format_ident!("{}With{}Context", ident, context.to_string())
                    } else {
                        ident
                    };
                    quote! { #new_ident }
                } else {
                    quote! { #token }
                }
            })
            .collect::<TokenStream>(),
        ContextAndFilterHandler::FilterNoContext(_) => TokenStream::from_iter(output_token_tree),
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(context) => {
            strip_sparse_prefix(output_token_tree, Some(*context))
        }
    }
}

pub fn fn_signature(
    part_of: PartOf,
    variant_ident: &Ident,
    url_parts: &[UrlPart],
    params_type: Option<&ParamsType>,
    request_type: RequestType,
    context_and_filter_handler: &ContextAndFilterHandler,
) -> TokenStream {
    let fn_name = fn_name(variant_ident, context_and_filter_handler);
    let url_params = fn_url_params(url_parts);
    let provided_param = fn_provided_param(part_of, params_type, request_type);
    let fields_param = fn_fields_param(context_and_filter_handler);
    quote! { fn #fn_name(&self, #url_params #provided_param #fields_param) }
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
    params_type: Option<&ParamsType>,
    request_type: RequestType,
) -> TokenStream {
    if let Some(params_type) = params_type {
        let tokens = {
            let params_type_token_stream = &params_type.tokens;
            quote! { params: #params_type_token_stream, }
        };
        match part_of {
            // Endpoints don't need the params type if it's a Post request because params will
            // be part of the body.
            PartOf::Endpoint => match request_type {
                crate::parse::RequestType::ContextualGet
                | crate::parse::RequestType::Delete
                | crate::parse::RequestType::Get => tokens,
                crate::parse::RequestType::Post => TokenStream::new(),
            },
            PartOf::RequestBuilder | PartOf::RequestExecutor => tokens,
        }
    } else {
        TokenStream::new()
    }
}

pub fn fn_fields_param(context_and_filter_handler: &ContextAndFilterHandler) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_) => TokenStream::new(),
        ContextAndFilterHandler::FilterTakeContextAsFunctionName(context, filter_by_type) => {
            let filter_by_type_token_stream = filter_by_type
                .tokens
                .clone()
                .into_iter()
                .map(|token| {
                    if let TokenTree::Ident(ident) = token {
                        let new_ident = if ident.to_string().starts_with(SPARSE_IDENT_PREFIX) {
                            // For example, given `SparseFooField`, it may be `SparseFooFieldWithEditContext`
                            format_ident!("{}With{}Context", ident, context.to_string())
                        } else {
                            ident
                        };
                        quote! { #new_ident }
                    } else {
                        quote! { #token }
                    }
                })
                .collect::<TokenStream>();
            quote! { fields: &[#filter_by_type_token_stream] }
        }
        ContextAndFilterHandler::FilterNoContext(filter_by_type) => {
            let filter_by_type_token_stream = &filter_by_type.tokens;
            quote! { fields: &[#filter_by_type_token_stream] }
        }
    }
}

pub fn fn_name(
    variant_ident: &Ident,
    context_and_filter_handler: &ContextAndFilterHandler,
) -> Ident {
    let basic_fn_name = format_ident!("{}", variant_ident.to_string().to_case(Case::Snake));
    match context_and_filter_handler {
        ContextAndFilterHandler::None => basic_fn_name,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(context) => format_ident!(
            "{}_with_{}_context",
            basic_fn_name,
            context.to_string().to_lowercase()
        ),
        ContextAndFilterHandler::FilterNoContext(_) => {
            format_ident!("filter_{}", basic_fn_name)
        }
        ContextAndFilterHandler::FilterTakeContextAsFunctionName(context, _) => {
            format_ident!(
                "filter_{}_with_{}_context",
                basic_fn_name,
                context.to_string().to_lowercase()
            )
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

fn fn_arg_provided_params(
    part_of: PartOf,
    params_type: Option<&ParamsType>,
    request_type: RequestType,
) -> TokenStream {
    if params_type.is_some() {
        let tokens = quote! { params, };
        match part_of {
            // Endpoints don't need the params type if it's a Post request because params will
            // be part of the body.
            PartOf::Endpoint => match request_type {
                crate::parse::RequestType::ContextualGet
                | crate::parse::RequestType::Delete
                | crate::parse::RequestType::Get => tokens,
                crate::parse::RequestType::Post => TokenStream::new(),
            },
            PartOf::RequestBuilder | PartOf::RequestExecutor => tokens,
        }
    } else {
        TokenStream::new()
    }
}

fn fn_arg_fields(context_and_filter_handler: &ContextAndFilterHandler) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_) => TokenStream::new(),
        ContextAndFilterHandler::FilterTakeContextAsFunctionName(_, _)
        | ContextAndFilterHandler::FilterNoContext(_) => quote! { fields, },
    }
}

pub fn fn_body_get_url_from_api_base_url(enum_ident: &Ident, url_parts: &[UrlPart]) -> TokenStream {
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
    quote! {
        let mut url = self.api_base_url.by_extending_and_splitting_by_forward_slash(Some(&#enum_ident::namespace()), [ #(#url_parts,)* ]);
    }
}

pub fn fn_body_get_url_from_endpoint(
    variant_ident: &Ident,
    url_parts: &[UrlPart],
    params_type: Option<&ParamsType>,
    request_type: RequestType,
    context_and_filter_handler: &ContextAndFilterHandler,
) -> TokenStream {
    let fn_name = fn_name(variant_ident, context_and_filter_handler);
    let fn_arg_url_parts = fn_arg_url_parts(url_parts);
    let fn_arg_provided_params =
        fn_arg_provided_params(PartOf::Endpoint, params_type, request_type);
    let fn_arg_fields = fn_arg_fields(context_and_filter_handler);

    quote! {
        let url = self.endpoint.#fn_name(#fn_arg_url_parts #fn_arg_provided_params #fn_arg_fields);
    }
}

pub fn fn_body_query_pairs(
    crate_ident: &Ident,
    params_type: Option<&ParamsType>,
    request_type: RequestType,
) -> TokenStream {
    match request_type {
        RequestType::ContextualGet | RequestType::Delete | RequestType::Get => {
            if let Some(params_type) = params_type {
                let is_option = if let Some(TokenTree::Ident(ref ident)) =
                    params_type.tokens.clone().into_iter().next()
                {
                    // TODO: This won't work with `std::option::Option` or `core::option::Option`
                    *ident == "Option"
                } else {
                    false
                };
                let append_query_pairs = quote! {
                    use #crate_ident::url_query::AppendUrlQueryPairs;
                    params.append_query_pairs(&mut url.query_pairs_mut());
                };
                if is_option {
                    quote! {
                        if let Some(params) = params {
                            #append_query_pairs
                        }
                    }
                } else {
                    quote! {
                        #append_query_pairs
                    }
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
    context_and_filter_handler: &ContextAndFilterHandler,
) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None
        | ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(_) => TokenStream::new(),
        ContextAndFilterHandler::FilterTakeContextAsFunctionName(_, _)
        | ContextAndFilterHandler::FilterNoContext(_) => quote! {
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
    context_and_filter_handler: &ContextAndFilterHandler,
) -> TokenStream {
    match context_and_filter_handler {
        ContextAndFilterHandler::None | ContextAndFilterHandler::FilterNoContext(_) => {
            TokenStream::new()
        }
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(context)
        | ContextAndFilterHandler::FilterTakeContextAsFunctionName(context, ..) => {
            let context = format_ident!("{}", context.to_string());
            quote! {
                url.query_pairs_mut().append_pair("context", #crate_ident::WpContext::#context.as_str());
            }
        }
    }
}

pub fn fn_body_build_request_from_url(
    params_type: Option<&ParamsType>,
    request_type: RequestType,
) -> TokenStream {
    match request_type {
        RequestType::ContextualGet | RequestType::Get => quote! {
            self.inner.get(url)
        },
        RequestType::Delete => quote! {
            self.inner.delete(url)
        },
        RequestType::Post => {
            if params_type.is_some() {
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
    params_type: Option<&ParamsType>,
    request_type: RequestType,
    context_and_filter_handler: &ContextAndFilterHandler,
) -> TokenStream {
    let fn_name = fn_name(variant_ident, context_and_filter_handler);
    let fn_arg_url_parts = fn_arg_url_parts(url_parts);
    let fn_arg_provided_params =
        fn_arg_provided_params(PartOf::RequestExecutor, params_type, request_type);
    let fn_arg_fields = fn_arg_fields(context_and_filter_handler);

    quote! {
        let request = self.request_builder.#fn_name(#fn_arg_url_parts #fn_arg_provided_params #fn_arg_fields);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::too_many_arguments)]
    use super::*;
    use crate::variant_attr::FilterByType;
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
    #[case(PartOf::Endpoint, None, RequestType::ContextualGet, "")]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ""
    )]
    #[case(
        PartOf::RequestBuilder,
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        "params : & UserCreateParams ,"
    )]
    #[case(
        PartOf::RequestExecutor,
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        "params : & UserCreateParams ,"
    )]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "params : & UserListParams ,"
    )]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "params : & UserListParams ,"
    )]
    fn test_fn_provided_param(
        #[case] part_of: PartOf,
        #[case] params_type: Option<ParamsType>,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_provided_param(part_of, params_type.as_ref(), request_type).to_string(),
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
    #[case(
        "ListContents",
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "list_contents_with_embed_context"
    )]
    #[case(
        "List",
        filter_take_context_as_argument(),
        "filter_list_with_edit_context"
    )]
    fn test_fn_name(
        #[case] ident: &str,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_name(&format_ident!("{}", ident), &context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, "")]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::View),
        ""
    )]
    #[case(
        filter_take_context_as_argument(),
        "fields : & [crate :: SparseUserFieldWithEditContext]"
    )]
    #[case(filter_no_context(), "fields : & [crate :: SparseUserField]")]
    fn test_fn_fields_param(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_fields_param(&context_and_filter_handler,).to_string(),
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
    #[case(PartOf::Endpoint, None, RequestType::ContextualGet, "")]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ""
    )]
    #[case(
        PartOf::RequestBuilder,
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        "params ,"
    )]
    #[case(
        PartOf::RequestExecutor,
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        "params ,"
    )]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "params ,"
    )]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "params ,"
    )]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserListParams"),
        RequestType::Delete,
        "params ,"
    )]
    #[case(
        PartOf::Endpoint,
        referenced_params_type("UserListParams"),
        RequestType::Post,
        ""
    )]
    fn test_fn_arg_provided_params(
        #[case] part_of: PartOf,
        #[case] params_type: Option<ParamsType>,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_arg_provided_params(part_of, params_type.as_ref(), request_type).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, "")]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        ""
    )]
    #[case(filter_take_context_as_argument(), "fields ,")]
    #[case(filter_no_context(), "fields ,")]
    fn test_fn_arg_fields(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_arg_fields(&context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(parse_quote!(crate::SparseUser), ContextAndFilterHandler::None, "crate :: User")]
    #[case(parse_quote!(crate::UserWithEditContext), ContextAndFilterHandler::None, "crate :: UserWithEditContext")]
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
    #[case(parse_quote!(SparseUser), filter_take_context_as_argument(), "SparseUserWithEditContext")]
    #[case(parse_quote!(Vec<SparseUser>), filter_no_context(), "Vec < SparseUser >")]
    fn test_output_type(
        #[case] output_token_stream: TokenStream,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            output_type(
                output_token_stream.into_iter().collect(),
                &context_and_filter_handler
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
        None,
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn create (& self ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Create"),
        url_static_users(),
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        filter_no_context(),
        "fn filter_create (& self , fields : & [crate :: SparseUserField])")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("Create"),
        url_static_users(),
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        filter_no_context(),
        "fn filter_create (& self , params : & UserCreateParams , fields : & [crate :: SparseUserField])")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("Create"),
        url_static_users(),
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        filter_no_context(),
        "fn filter_create (& self , params : & UserCreateParams , fields : & [crate :: SparseUserField])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Delete"),
        url_users_with_user_id(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "fn delete (& self , user_id : & UserId , params : & UserDeleteParams ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Delete"),
        url_users_with_user_id(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        filter_no_context(),
        "fn filter_delete (& self , user_id : & UserId , params : & UserDeleteParams , fields : & [crate :: SparseUserField])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("DeleteMe"),
        url_static_users(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "fn delete_me (& self , params : & UserDeleteParams ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("List"),
        url_static_users(),
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        filter_take_context_as_argument(),
        "fn filter_list_with_edit_context (& self , params : & UserListParams , fields : & [crate :: SparseUserFieldWithEditContext])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        None,
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "fn retrieve_with_embed_context (& self , user_id : & UserId ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        None,
        RequestType::ContextualGet,
        filter_take_context_as_argument(),
        "fn filter_retrieve_with_edit_context (& self , user_id : & UserId , fields : & [crate :: SparseUserFieldWithEditContext])")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn update (& self , user_id : & UserId ,)")]
    #[case(
        PartOf::Endpoint,
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        filter_no_context(),
        "fn filter_update (& self , user_id : & UserId , fields : & [crate :: SparseUserField])")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn update (& self , user_id : & UserId , params : & UserUpdateParams ,)")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "fn update (& self , user_id : & UserId , params : & UserUpdateParams ,)")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        filter_no_context(),
        "fn filter_update (& self , user_id : & UserId , params : & UserUpdateParams , fields : & [crate :: SparseUserField])")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        filter_no_context(),
        "fn filter_update (& self , user_id : & UserId , params : & UserUpdateParams , fields : & [crate :: SparseUserField])")]
    #[case(
        PartOf::RequestBuilder,
        format_ident!("List"),
        url_static_users(),
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "fn list_with_edit_context (& self , params : & UserListParams ,)")]
    #[case(
        PartOf::RequestExecutor,
        format_ident!("List"),
        url_static_users(),
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "fn list_with_edit_context (& self , params : & UserListParams ,)")]
    fn test_fn_signature(
        #[case] part_of: PartOf,
        #[case] variant_ident: Ident,
        #[case] url_parts: Vec<UrlPart>,
        #[case] params_type: Option<ParamsType>,
        #[case] request_type: RequestType,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_signature(
                part_of,
                &variant_ident,
                &url_parts,
                params_type.as_ref(),
                request_type,
                &context_and_filter_handler,
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . create () ;")]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        filter_no_context(),
        "let url = self . endpoint . filter_create (fields ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . delete (user_id , params ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        filter_no_context(),
        "let url = self . endpoint . filter_delete (user_id , params , fields ,) ;")]
    #[case(
        format_ident!("DeleteMe"),
        url_static_users(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . delete_me (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "let url = self . endpoint . list_with_edit_context (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        filter_take_context_as_argument(),
        "let url = self . endpoint . filter_list_with_edit_context (params , fields ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        None,
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "let url = self . endpoint . retrieve_with_embed_context (user_id ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        None,
        RequestType::ContextualGet,
        filter_take_context_as_argument(),
        "let url = self . endpoint . filter_retrieve_with_edit_context (user_id , fields ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let url = self . endpoint . update (user_id ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        filter_no_context(),
        "let url = self . endpoint . filter_update (user_id , fields ,) ;")]
    fn test_fn_body_get_url_from_endpoint(
        #[case] variant_ident: Ident,
        #[case] url_parts: Vec<UrlPart>,
        #[case] params_type: Option<ParamsType>,
        #[case] request_type: RequestType,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_get_url_from_endpoint(
                &variant_ident,
                &url_parts,
                params_type.as_ref(),
                request_type,
                &context_and_filter_handler
            )
            .to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(
        url_static_users(),
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash (Some (& Foo :: namespace ()) , [\"users\" ,]) ;"
    )]
    #[case(
        url_users_with_user_id(),
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash (Some (& Foo :: namespace ()) , [\"users\" , & user_id . to_string () ,]) ;"
    )]
    #[case(
        url_users_with_user_id(),
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash (Some (& Foo :: namespace ()) , [\"users\" , & user_id . to_string () ,]) ;"
    )]
    #[case(
        vec![UrlPart::Dynamic("user_id".to_string()), UrlPart::Dynamic("user_type".to_string())],
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash (Some (& Foo :: namespace ()) , [& user_id . to_string () , & user_type . to_string () ,]) ;"
    )]
    #[case(
        vec![UrlPart::Static("users".to_string()), UrlPart::Dynamic("user_id".to_string()), UrlPart::Dynamic("user_type".to_string()), ],
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash (Some (& Foo :: namespace ()) , [\"users\" , & user_id . to_string () , & user_type . to_string () ,]) ;"
    )]
    #[case(
        vec![UrlPart::Static("users".to_string()), UrlPart::Static("me".to_string()), UrlPart::Dynamic("user_id".to_string()), UrlPart::Dynamic("user_type".to_string()), ],
        "let mut url = self . api_base_url . by_extending_and_splitting_by_forward_slash (Some (& Foo :: namespace ()) , [\"users\" , \"me\" , & user_id . to_string () , & user_type . to_string () ,]) ;"
    )]
    fn test_fn_body_get_url_from_api_base_url(
        #[case] url_parts: Vec<UrlPart>,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_get_url_from_api_base_url(&format_ident!("Foo"), &url_parts).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(None, RequestType::ContextualGet, "")]
    #[case(
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "use crate :: url_query :: AppendUrlQueryPairs ; params . append_query_pairs (& mut url . query_pairs_mut ()) ;"
    )]
    #[case(
        option_referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "if let Some (params) = params { use crate :: url_query :: AppendUrlQueryPairs ; params . append_query_pairs (& mut url . query_pairs_mut ()) ; }"
    )]
    #[case(option_referenced_params_type("UserListParams"), RequestType::Post, "")]
    fn test_fn_body_query_pairs(
        #[case] params: Option<ParamsType>,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        let crate_ident = format_ident!("crate");
        assert_eq!(
            fn_body_query_pairs(&crate_ident, params.as_ref(), request_type).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, true)]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        true
    )]
    #[case(filter_take_context_as_argument(), false)]
    #[case(filter_no_context(), false)]
    fn test_fn_body_fields_query_pairs(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] is_empty: bool,
    ) {
        // Test if the `_fields` query pair is included or not
        // Since this query pair is static, there is no need to compare the string value
        let crate_ident = format_ident!("crate");
        assert_eq!(
            fn_body_fields_query_pairs(&crate_ident, &context_and_filter_handler).is_empty(),
            is_empty
        );
    }

    #[rstest]
    #[case(ContextAndFilterHandler::None, "")]
    #[case(
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "url . query_pairs_mut () . append_pair (\"context\" , crate :: WpContext :: Edit . as_str ()) ;"
    )]
    #[case(
        filter_take_context_as_argument(),
        "url . query_pairs_mut () . append_pair (\"context\" , crate :: WpContext :: Edit . as_str ()) ;"
    )]
    #[case(filter_no_context(), "")]
    fn test_fn_body_context_query_pairs(
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        let crate_ident = format_ident!("crate");
        assert_eq!(
            fn_body_context_query_pairs(&crate_ident, &context_and_filter_handler).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(None, RequestType::ContextualGet, "self . inner . get (url)")]
    #[case(
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        "self . inner . get (url)"
    )]
    #[case(None, RequestType::Delete, "self . inner . delete (url)")]
    #[case(
        referenced_params_type("UserListParams"),
        RequestType::Delete,
        "self . inner . delete (url)"
    )]
    #[case(None, RequestType::Post, "self . inner . post (url)")]
    #[case(
        referenced_params_type("UserListParams"),
        RequestType::Post,
        "self . inner . post (url , params)"
    )]
    fn test_fn_body_build_request_from_url(
        #[case] params: Option<ParamsType>,
        #[case] request_type: RequestType,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_build_request_from_url(params.as_ref(), request_type).to_string(),
            expected_str
        );
    }

    #[rstest]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . create (params ,) ;")]
    #[case(
        format_ident!("Create"),
        url_static_users(),
        referenced_params_type("UserCreateParams"),
        RequestType::Post,
        filter_no_context(),
        "let request = self . request_builder . filter_create (params , fields ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . delete (user_id , params ,) ;")]
    #[case(
        format_ident!("Delete"),
        url_users_with_user_id(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        filter_no_context(),
        "let request = self . request_builder . filter_delete (user_id , params , fields ,) ;")]
    #[case(
        format_ident!("DeleteMe"),
        url_static_users(),
        referenced_params_type("UserDeleteParams"),
        RequestType::Delete,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . delete_me (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Edit),
        "let request = self . request_builder . list_with_edit_context (params ,) ;")]
    #[case(
        format_ident!("List"),
        url_static_users(),
        referenced_params_type("UserListParams"),
        RequestType::ContextualGet,
        filter_take_context_as_argument(),
        "let request = self . request_builder . filter_list_with_edit_context (params , fields ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        None,
        RequestType::ContextualGet,
        ContextAndFilterHandler::NoFilterTakeContextAsFunctionName(WpContext::Embed),
        "let request = self . request_builder . retrieve_with_embed_context (user_id ,) ;")]
    #[case(
        format_ident!("Retrieve"),
        url_users_with_user_id(),
        None,
        RequestType::ContextualGet,
        filter_take_context_as_argument(),
        "let request = self . request_builder . filter_retrieve_with_edit_context (user_id , fields ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        ContextAndFilterHandler::None,
        "let request = self . request_builder . update (user_id , params ,) ;")]
    #[case(
        format_ident!("Update"),
        url_users_with_user_id(),
        referenced_params_type("UserUpdateParams"),
        RequestType::Post,
        filter_no_context(),
        "let request = self . request_builder . filter_update (user_id , params , fields ,) ;")]
    fn test_fn_body_get_request_from_request_builder(
        #[case] variant_ident: Ident,
        #[case] url_parts: Vec<UrlPart>,
        #[case] params_type: Option<ParamsType>,
        #[case] request_type: RequestType,
        #[case] context_and_filter_handler: ContextAndFilterHandler,
        #[case] expected_str: &str,
    ) {
        assert_eq!(
            fn_body_get_request_from_request_builder(
                &variant_ident,
                &url_parts,
                params_type.as_ref(),
                request_type,
                &context_and_filter_handler
            )
            .to_string(),
            expected_str
        );
    }

    fn referenced_params_type(str: &str) -> Option<ParamsType> {
        let ident = format_ident!("{}", str);
        Some(ParamsType {
            tokens: quote! { & #ident },
        })
    }

    fn option_referenced_params_type(str: &str) -> Option<ParamsType> {
        let ident = format_ident!("{}", str);
        Some(ParamsType {
            tokens: quote! { Option< & #ident > },
        })
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

    fn filter_take_context_as_argument() -> ContextAndFilterHandler {
        ContextAndFilterHandler::FilterTakeContextAsFunctionName(
            WpContext::Edit,
            FilterByType {
                tokens: quote! { crate::SparseUserField },
            },
        )
    }

    fn filter_no_context() -> ContextAndFilterHandler {
        ContextAndFilterHandler::FilterNoContext(FilterByType {
            tokens: quote! { crate::SparseUserField },
        })
    }
}
