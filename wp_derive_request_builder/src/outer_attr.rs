use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Meta, Result,
};

#[derive(Debug, Clone)]
pub struct ErrorTypeAttr {
    pub tokens: TokenStream,
}

#[derive(Debug, Clone)]
pub struct OuterAttr {
    pub error_type_attr: ErrorTypeAttr,
}

impl Parse for OuterAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        let error_type_attr = {
            let first_attr = attrs
                .first()
                .ok_or(OuterAttrParseError::MissingErrorTypeAttr.into_syn_error(input.span()))?;
            let Meta::List(ref meta_list) = first_attr.meta else {
                return Err(
                    OuterAttrParseError::MissingErrorTypeAttr.into_syn_error(first_attr.span())
                );
            };

            if meta_list.path.segments.len() != 1 {
                return Err(
                    OuterAttrParseError::MissingErrorTypeAttr.into_syn_error(first_attr.span())
                );
            }

            let s = meta_list
                .path
                .segments
                .first()
                .expect("Already verified that there is only one segment");

            if s.ident.to_string() != "ErrorType" {
                return Err(
                    OuterAttrParseError::MissingErrorTypeAttr.into_syn_error(first_attr.span())
                );
            }

            if meta_list.tokens.is_empty() {
                return Err(
                    OuterAttrParseError::ErrorTypeAttrHasNoTokens.into_syn_error(first_attr.span())
                );
            }

            ErrorTypeAttr {
                tokens: meta_list.tokens.clone(),
            }
        };

        Ok(Self { error_type_attr })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OuterAttrParseError {
    #[error("Expecting #[ErrorType(\"_error_type_\")] - Path is missing")]
    ErrorTypeAttrHasNoTokens,
    #[error("Missing #[ErrorType(\"_error_type_\")] attribute")]
    MissingErrorTypeAttr,
}

impl OuterAttrParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
