use proc_macro2::{Literal, TokenStream, TokenTree};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Ident, Meta, MetaList, Result,
};

#[derive(Debug, Clone)]
pub struct NamespaceAttr {
    pub token: TokenTree,
}

impl TryFrom<TokenStream> for NamespaceAttr {
    type Error = OuterAttrParseError;

    fn try_from(value: TokenStream) -> std::result::Result<Self, Self::Error> {
        let mut iter = value.into_iter();
        if let Some(first) = iter.next() {
            if iter.next().is_some() {
                // Has more than one token
                Err(OuterAttrParseError::WrongNamespaceAttrFormat)
            } else if let TokenTree::Literal(_) = first {
                Ok(Self { token: first })
            } else {
                // Is not a literal
                Err(OuterAttrParseError::NamespaceAttrIsNotLiteral)
            }
        } else {
            // Doesn't have any tokens
            Err(OuterAttrParseError::WrongNamespaceAttrFormat)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SparseFieldAttr {
    pub tokens: TokenStream,
}

#[derive(Debug, Clone)]
pub struct OuterAttr {
    pub namespace_attr: NamespaceAttr,
    pub sparse_field_attr: SparseFieldAttr,
}

impl Parse for OuterAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let (sparse_field, namespace) = attrs.into_iter().try_fold((None, None), |acc, a| {
            let error_span = a.span();
            if let Meta::List(meta_list) = a.meta {
                if meta_list.path.segments.len() != 1 {
                    Err(OuterAttrParseError::UnexpectedAttrPathSegmentCount
                        .into_syn_error(error_span))
                } else {
                    let s = meta_list
                        .path
                        .segments
                        .first()
                        .expect("Already verified that there is only one segment");

                    match s.ident.to_string().as_str() {
                        "SparseField" => Ok((Some(meta_list.tokens), acc.1)),
                        "Namespace" => Ok((acc.0, Some(meta_list.tokens))),
                        // Unrecognized attribute may belong to another Derive macro, so we need
                        // to ignore and not return an error
                        _ => Ok(acc),
                    }
                }
            } else {
                Err(OuterAttrParseError::WrongOuterAttrFormat.into_syn_error(error_span))
            }
        })?;
        let sparse_field_attr = sparse_field
            .map(|tokens| SparseFieldAttr { tokens })
            .ok_or(OuterAttrParseError::MissingSparseFieldAttr.into_syn_error(input.span()))?;
        let namespace_attr = namespace
            .ok_or(OuterAttrParseError::MissingNamespaceAttr.into_syn_error(input.span()))
            .and_then(|t| NamespaceAttr::try_from(t).map_err(|e| e.into_syn_error(input.span())))?;

        Ok(Self {
            namespace_attr,
            sparse_field_attr,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OuterAttrParseError {
    #[error("Expecting #[Namespace(\"_path_\")] - Did you forget (\"\")?")]
    NamespaceAttrIsNotLiteral,
    #[error("Expecting #[Namespace(\"_path_\")] - Found extra tokens")]
    NamespaceAttrHasMultipleTokens,
    #[error("Missing #[Namespace(\"_path_\")]")]
    MissingNamespaceAttr,
    #[error("Missing #[SparseField(_field_type_)]")]
    MissingSparseFieldAttr,
    #[error("Only SparseField & Namespace attributes only have one path segment")]
    UnexpectedAttrPathSegmentCount,
    #[error("Expecting #[SparseField(_field_type_)] & #[Namespace(\"_path_\")]")]
    WrongOuterAttrFormat,
    #[error("Expecting #[Namespace(\"_path_\")]")]
    WrongNamespaceAttrFormat,
}

impl OuterAttrParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
