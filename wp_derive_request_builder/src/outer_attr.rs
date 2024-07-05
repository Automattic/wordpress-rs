use proc_macro2::{Literal, Span, TokenStream, TokenTree};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Ident, Meta, MetaList, Result,
};

#[derive(Debug, Clone)]
pub struct NamespaceAttr {
    pub token: TokenTree,
}

impl NamespaceAttr {
    fn new(tokens: TokenStream, attr_span: Span) -> Result<Self> {
        let tokens_span = tokens.span();
        let mut iter = tokens.into_iter();
        if let Some(first) = iter.next() {
            if iter.next().is_some() {
                Err(OuterAttrParseError::NamespaceAttrHasMultipleTokens.into_syn_error(tokens_span))
            } else if let TokenTree::Literal(_) = first {
                Ok(Self { token: first })
            } else {
                Err(OuterAttrParseError::NamespaceAttrIsNotLiteral.into_syn_error(first.span()))
            }
        } else {
            Err(OuterAttrParseError::NamespaceAttrHasNoTokens.into_syn_error(attr_span))
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

        let (sparse_field, namespace) = attrs.into_iter().fold((None, None), |(acc), a| {
            let attr_span = a.span();
            let Meta::List(meta_list) = a.meta else {
                return acc;
            };
            if meta_list.path.segments.len() == 1 {
                let s = meta_list
                    .path
                    .segments
                    .first()
                    .expect("Already verified that there is only one segment");

                match s.ident.to_string().as_str() {
                    "SparseField" => (Some((meta_list.tokens, attr_span)), acc.1),
                    "Namespace" => (acc.0, Some((meta_list.tokens, attr_span))),
                    // Unrecognized attributes may belong to another proc macro, so we need
                    // to ignore them and not return an error
                    _ => acc,
                }
            } else {
                acc
            }
        });
        let sparse_field_attr = sparse_field
            .map(|(tokens, _)| SparseFieldAttr { tokens })
            .ok_or(OuterAttrParseError::MissingSparseFieldAttr.into_syn_error(input.span()))?;
        let namespace_attr = namespace
            .ok_or(OuterAttrParseError::MissingNamespaceAttr.into_syn_error(input.span()))
            .and_then(|(t, attr_span)| NamespaceAttr::new(t, attr_span))?;

        Ok(Self {
            namespace_attr,
            sparse_field_attr,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OuterAttrParseError {
    #[error("Expecting #[Namespace(\"_path_\")] - Try wrapping the path in \"\"")]
    NamespaceAttrIsNotLiteral,
    #[error(
        "Expecting #[Namespace(\"_path_\")] - Path should be a single literal separated by '/'"
    )]
    NamespaceAttrHasMultipleTokens,
    #[error("Expecting #[Namespace(\"_path_\")] - Path is missing")]
    NamespaceAttrHasNoTokens,
    #[error("Missing #[Namespace(\"_path_\")] attribute")]
    MissingNamespaceAttr,
    #[error("Missing #[SparseField(_field_type_)] attribute")]
    MissingSparseFieldAttr,
}

impl OuterAttrParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
