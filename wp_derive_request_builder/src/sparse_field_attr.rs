use proc_macro2::{TokenStream, TokenTree};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Ident, Meta, MetaList, Result,
};

#[derive(Debug, Clone)]
pub struct SparseFieldAttr {
    pub tokens: TokenStream,
}

impl Parse for SparseFieldAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr = {
            let attrs = Attribute::parse_outer(input)?;
            if attrs.is_empty() {
                return Err(
                    SparseFieldParseError::MissingSparseFieldAttr.into_syn_error(input.span())
                );
            } else if attrs.len() > 1 {
                return Err(
                    SparseFieldParseError::MoreThanOneOuterAttr.into_syn_error(input.span())
                );
            }
            attrs
                .first()
                .expect("Already verified that there is a single attr")
                .to_owned()
        };

        if let Meta::List(MetaList { tokens, .. }) = attr.meta {
            Ok(Self { tokens })
        } else {
            Err(SparseFieldParseError::WrongFormat.into_syn_error(attr.span()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SparseFieldParseError {
    #[error("Missing #[SparseField(_field_type_)]")]
    MissingSparseFieldAttr,
    #[error("Only a single #[SparseField(_field_type_)] attribute is supported")]
    MoreThanOneOuterAttr,
    #[error("Expecting #[SparseField(_field_type_)]")]
    WrongFormat,
}

impl SparseFieldParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
