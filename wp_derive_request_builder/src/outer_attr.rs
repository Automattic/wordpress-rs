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
        if attrs.is_empty() {
            return Err(OuterAttrParseError::NoOuterAttrs.into_syn_error(input.span()));
        } else if attrs.len() > 2 {
            return Err(OuterAttrParseError::UnexpectedNumberOfAttrs.into_syn_error(input.span()));
        }

        let pairs = attrs
            .into_iter()
            .map(|a| {
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
                        Ok((s.ident.to_string(), meta_list.tokens))
                    }
                } else {
                    Err(OuterAttrParseError::WrongOuterAttrFormat.into_syn_error(error_span))
                }
            })
            .collect::<Result<Vec<(String, TokenStream)>>>()?;
        let (sparse_field, namespace) =
            pairs
                .into_iter()
                .try_fold((None, None), |acc, (k, tokens)| match k.as_str() {
                    "SparseField" => Ok((Some(tokens), acc.1)),
                    "Namespace" => Ok((acc.0, Some(tokens))),
                    _ => Err(OuterAttrParseError::UnexpectedAttr.into_syn_error(input.span())),
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
    #[error("Missing #[SparseField(_field_type_) & #[Namespace(\"_path_\")] attributes]")]
    NoOuterAttrs,
    #[error("Missing #[Namespace(\"_path_\")]")]
    MissingNamespaceAttr,
    #[error("Missing #[SparseField(_field_type_)]")]
    MissingSparseFieldAttr,
    #[error(
        "Only #[SparseField(_field_type_)] & #[Namespace(\"_path_\")] attributes are supported"
    )]
    UnexpectedNumberOfAttrs,
    #[error("Only SparseField & Namespace attributes only have one path segment")]
    UnexpectedAttrPathSegmentCount,
    #[error("Expecting #[SparseField(_field_type_)] & #[Namespace(\"_path_\")]")]
    WrongOuterAttrFormat,
    #[error("Expecting #[Namespace(\"_path_\")]")]
    WrongNamespaceAttrFormat,
    #[error(
        "Only #[SparseField(_field_type_)] & #[Namespace(\"_path_\")] attributes are supported"
    )]
    UnexpectedAttr,
}

impl OuterAttrParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
