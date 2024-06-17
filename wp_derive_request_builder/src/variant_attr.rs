use proc_macro2::{TokenStream, TokenTree};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Ident, MetaList,
};

use crate::parse::RequestType;

#[derive(Debug)]
pub struct ParsedVariantAttribute {
    request_type: RequestType,
    url: String,
    params: Option<Vec<TokenTree>>,
    output: Vec<TokenTree>,
}

impl ParsedVariantAttribute {
    // Parses the attribute and finds the [syn::MetaList]
    //
    // Errors:
    // * `MissingAttr` if there are no attributes
    // * `MoreThanOneOuterAttr` if there are multiple attributes
    // * `MetaInWrongFormat` if the attribute is not a [syn::Meta::List]
    fn meta_list(input: ParseStream) -> syn::Result<MetaList> {
        let attrs = Attribute::parse_outer(input)?;
        if attrs.is_empty() {
            return Err(ItemVariantAttributeParseError::MissingAttr.into_syn_error(input.span()));
        } else if attrs.len() > 1 {
            return Err(
                ItemVariantAttributeParseError::MoreThanOneOuterAttr.into_syn_error(input.span())
            );
        }
        let attr = attrs
            .first()
            .expect("Already verified that there is a single attr")
            .to_owned();

        if let syn::Meta::List(meta_list) = attr.meta {
            Ok(meta_list)
        } else {
            Err(
                ItemVariantAttributeParseError::MetaInWrongFormat { meta: attr.meta }
                    .into_syn_error(input.span()),
            )
        }
    }

    // Finds the request type from [syn::MetaList.path]
    //
    // In practice, no errors would be thrown here because the supported request types has to be
    // declared while declaring the proc macro. However, if the expected format were to be changed
    // having proper error handling here will help.
    //
    // Errors:
    // * `UnsupportedRequestType` if the meta list doesn't have exactly one path segment
    // * `UnsupportedRequestType` if the path can't be converted to a [RequestType]
    fn request_type(meta_list: &MetaList) -> syn::Result<RequestType> {
        let build_err =
            |span| Err(ItemVariantAttributeParseError::UnsupportedRequestType.into_syn_error(span));
        if meta_list.path.segments.len() != 1 {
            return build_err(meta_list.path.span());
        }
        let path_segment = meta_list
            .path
            .segments
            .first()
            .expect("Already validated that there is only one segment");

        match path_segment.ident.to_string().as_str() {
            "contextual_get" => Ok(RequestType::ContextualGet),
            "delete" => Ok(RequestType::Delete),
            "get" => Ok(RequestType::Get),
            "post" => Ok(RequestType::Post),
            _ => build_err(path_segment.ident.span()),
        }
    }

    // Splits the original [TokenStream] into multiple [TokenTree] lists; using `,` as a separator
    //
    // ```
    // #[derive(WpDerivedRequest)]
    // #[SparseField(crate::SparseUserField)]
    // enum UsersRequest {
    //     #[contextual_get(url = "/users", params = &UserListParams, output = Vec<SparseUser>)]
    //     List,
    // }
    // ```
    //
    // In the above example: (as pseudocode)
    // * Input: `url = "/users", params = &UserListParams, output = Vec<SparseUser>`
    // * Output: `vec!["url = \"users\"", "params = &UserListParams", "output = Vec<SparseUser>"]`
    fn split_by_comma(tokens: TokenStream) -> Vec<Vec<TokenTree>> {
        let mut collection = vec![];
        let mut temp_v = vec![];
        for t in tokens {
            if let TokenTree::Punct(ref p) = t {
                if p.as_char() == ',' {
                    collection.push(temp_v);
                    temp_v = vec![];
                    continue;
                }
            }
            temp_v.push(t);
        }
        // Tokens after the final ','
        if !temp_v.is_empty() {
            collection.push(temp_v);
        }
        collection
    }

    // Splits each token list in a given list of token lists; using `=` as a separator
    //
    // ```
    // #[derive(WpDerivedRequest)]
    // #[SparseField(crate::SparseUserField)]
    // enum UsersRequest {
    //     #[contextual_get(url = "/users", params = &UserListParams, output = Vec<SparseUser>)]
    //     List,
    // }
    // ```
    //
    // In the above example: (as pseudocode)
    // * Input: `vec!["url = \"users\"", "params = &UserListParams", "output = Vec<SparseUser>"]`
    // * Output: `vec![(url, "users"), (params, &UserListParams), (output, Vec<SparseUser>)]`
    //
    // How it works:
    // * Since a specific set of keys is expected - "url", "params" or "output", it's certain that
    // keys are `syn::Ident`s. Furthermore, the input is comma separated list, so it's certain that
    // the first token is the key.
    // * Since the first token is guaranteed to be a single `syn::Ident`, the second token is
    // guaranteed to be an `=` sign.
    // * The remaining tokens represent the "value". There needs to be at least one token.
    // * This parser does not ensure that the parsed keys are one of the expected types nor does it
    // ensure that the "value" tokens would represent a valid value. The former should be handled
    // as a separate step, the latter can't be "ensured" but common issues could be raised in a
    // separate step.
    //
    // Errors:
    // * `ExpectingKeyValuePairs` if the first token is not a `syn::Ident`
    // * `ExpectingKeyValuePairs` if the second token is not a `=` sign
    // * `ExpectingKeyValuePairs` if there aren't any tokens after `=` sign
    fn split_by_equals(
        token_group: Vec<Vec<TokenTree>>,
    ) -> syn::Result<Vec<(Ident, Vec<TokenTree>)>> {
        let build_err =
            |span| Err(ItemVariantAttributeParseError::ExpectingKeyValuePairs.into_syn_error(span));
        token_group
            .into_iter()
            .map(|tokens| {
                let mut tokens_iter = tokens.into_iter();
                // First token should be an Ident matching to "url", "params" or "output"
                let first_token = tokens_iter.next();
                let ident = if let Some(TokenTree::Ident(ident)) = first_token {
                    ident
                } else {
                    return build_err(first_token.span());
                };

                // Second token should be `=` sign
                let second_token = tokens_iter.next();
                if let Some(TokenTree::Punct(ref p)) = second_token {
                    if p.as_char() != '=' {
                        return Err(ItemVariantAttributeParseError::NotEqualsSign
                            .into_syn_error(second_token.span()));
                    }
                } else {
                    return build_err(second_token.span());
                }

                // If there are no tokens left after `=` sign, it's not formatted correctly
                if tokens_iter.len() == 0 {
                    // We don't have a great span to raise this error, so we use the `=` sign
                    return build_err(second_token.span());
                }

                // Group remaining tokens
                Ok((ident, tokens_iter.collect()))
            })
            .collect()
    }
}

impl Parse for ParsedVariantAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let meta_list = Self::meta_list(input)?;
        let meta_list_span = meta_list.span();
        let request_type = Self::request_type(&meta_list)?;
        let split_by_comma = ParsedVariantAttribute::split_by_comma(meta_list.tokens);
        let pair_vec = ParsedVariantAttribute::split_by_equals(split_by_comma)?;

        let mut url_tokens = None;
        let mut params_tokens = None;
        let mut output_tokens = None;

        for (ident, tokens) in pair_vec.into_iter() {
            match ident.to_string().as_str() {
                "url" => url_tokens = Some(tokens),
                "params" => params_tokens = Some(tokens),
                "output" => output_tokens = Some(tokens),
                _ => {
                    return Err(ItemVariantAttributeParseError::ExpectingKeyValuePairs
                        .into_syn_error(meta_list_span));
                }
            }
        }

        let url_str = if let Some(mut url_tokens) = url_tokens {
            // There should only be one literal token
            if url_tokens.len() != 1 {
                return Err(
                    ItemVariantAttributeParseError::UrlShouldBeLiteral.into_syn_error(input.span())
                );
            }
            let first_token = url_tokens
                .first_mut()
                .expect("Already verified that there is only one url token")
                .to_owned();

            if let TokenTree::Literal(lit) = first_token {
                lit
            } else {
                return Err(ItemVariantAttributeParseError::UrlShouldBeLiteral
                    .into_syn_error(first_token.span()));
            }
        } else {
            return Err(ItemVariantAttributeParseError::MissingUrl.into_syn_error(meta_list_span));
        };

        let output = output_tokens.ok_or_else(|| {
            ItemVariantAttributeParseError::MissingOutput.into_syn_error(input.span())
        })?;

        Ok(Self {
            request_type,
            url: url_str.to_string(),
            params: params_tokens,
            output,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ItemVariantAttributeParseError {
    #[error("Missing variant attribute")]
    MissingAttr,
    #[error("Only a single attribute is supported")]
    MoreThanOneOuterAttr,
    #[error("Expecting a syn::Meta::List found {:?}", meta)]
    MetaInWrongFormat { meta: syn::Meta },
    #[error("Expecting key value pairs (url = \"\", params = FooParam, output = FooOutput)")]
    ExpectingKeyValuePairs,
    #[error("Did you mean '='?")]
    NotEqualsSign,
    #[error("Missing (url = \"/foo\")")]
    MissingUrl,
    #[error("Url should be set as a String: (url = \"/foo\")")]
    UrlShouldBeLiteral,
    #[error("Missing (output = crate::Foo)")]
    MissingOutput,
    #[error("Only 'contextual_get', 'get', 'post' & 'delete' are supported")]
    UnsupportedRequestType,
}

impl ItemVariantAttributeParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
