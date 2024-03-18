use std::{fmt::Display, slice::Iter, str::FromStr};

use const_format::formatcp;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Ident};

const IDENT_PREFIX: &str = "Sparse";
const ERROR_MISSING_SPARSE_PREFIX_FROM_WP_CONTEXTUAL: &str = formatcp!("WPContextual types need to start with '{}' prefix. This prefix will be removed from the generated Structs, so it needs to be followed up with a proper Rust type name, starting with an uppercase letter.", IDENT_PREFIX);
const ERROR_MISSING_SPARSE_PREFIX_FROM_WP_CONTEXTUAL_FIELD: &str = formatcp!(
    "WPContextualField field types need to start with '{}' prefix",
    IDENT_PREFIX
);
const ERROR_EMPTY_RESULT: &str =
    "WPContextual didn't generate anything. Did you forget to add #[WPContext] attribute?";
const ERROR_WP_CONTEXTUAL_FIELD_WITHOUT_ANY_WP_CONTEXTS: &str =
    "#[WPContextualField] doesn't have any contexts. Did you forget to add #[WPContext] attribute?";

#[proc_macro_derive(WPContextual, attributes(WPContext, WPContextualField))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);
    let original_ident = &ast.ident;
    let ident_name_without_prefix = match ident_name_without_prefix(
        original_ident,
        ERROR_MISSING_SPARSE_PREFIX_FROM_WP_CONTEXTUAL,
    ) {
        Ok(ident) => ident,
        Err(e) => return e.into_compile_error().into(),
    };
    let fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma> =
        if let syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
            ..
        }) = ast.data
        {
            named
        } else {
            unimplemented!("Only implemented for Structs for now");
        };

    let parsed_fields = match parse_field_attrs(fields.iter()) {
        Ok(p) => p,
        Err(e) => return syn::Error::from(e).to_compile_error().into(),
    };
    if let Some(pf) = parsed_fields
        .iter()
        .filter(|pf| {
            pf.parsed_attrs
                .contains(&WPParsedAttr::ParsedWPContextualField)
        })
        .find(|pf| {
            // Find any field that has #[WPContextualField] attribute, but not #[WPContext]
            // attribute
            !pf.parsed_attrs.iter().any(|pf| match pf {
                WPParsedAttr::ParsedWPContext { contexts } => !contexts.is_empty(),
                _ => false,
            })
        })
    {
        return syn::Error::new(
            pf.field.span(),
            ERROR_WP_CONTEXTUAL_FIELD_WITHOUT_ANY_WP_CONTEXTS,
        )
        .to_compile_error()
        .into();
    };

    let contextual_token_streams = WPContextAttr::iter().map(|context_attr| {
        let cname = ident_name_for_context(&ident_name_without_prefix, context_attr);
        let cident = Ident::new(&cname, original_ident.span());
        let cfields = parsed_fields
            .iter()
            .filter(|pf| {
                pf.parsed_attrs.iter().any(|parsed_attr| {
                    if let WPParsedAttr::ParsedWPContext { contexts } = parsed_attr {
                        contexts.iter().any(|c| c == context_attr)
                    } else {
                        false
                    }
                })
            })
            .map(|pf| {
                let f = &pf.field;
                let is_wp_contextual_field = f.attrs.iter().any(|attr| {
                    attr.path()
                        .segments
                        .iter()
                        .any(|s| is_wp_contextual_field_ident(&s.ident))
                });
                let mut new_type = extract_inner_type_of_option(&f.ty).unwrap_or(f.ty.clone());
                if is_wp_contextual_field {
                    new_type = contextual_field_type(&new_type, context_attr)?;
                }
                Ok::<syn::Field, syn::Error>(syn::Field {
                    // Remove the WPContext & WPContextualField attributes from the generated field
                    attrs: pf
                        .parsed_attrs
                        .iter()
                        .filter_map(|parsed_attr| {
                            if let WPParsedAttr::ExternalAttr { attr } = parsed_attr {
                                Some(attr.to_owned())
                            } else {
                                None
                            }
                        })
                        .collect(),
                    vis: f.vis.clone(),
                    mutability: syn::FieldMutability::None,
                    ident: f.ident.clone(),
                    colon_token: f.colon_token,
                    ty: new_type,
                })
            })
            .collect::<Result<Vec<syn::Field>, syn::Error>>()?;
        if !cfields.is_empty() {
            Ok(quote! {
                #[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record)]
                pub struct #cident {
                    #(#cfields,)*
                }
            }
            .into())
        } else {
            Ok(proc_macro::TokenStream::new())
        }
    });
    contextual_token_streams
        .collect::<Result<Vec<TokenStream>, syn::Error>>()
        .map(TokenStream::from_iter)
        .and_then(|t: TokenStream| {
            if t.is_empty() {
                Err(syn::Error::new(original_ident.span(), ERROR_EMPTY_RESULT))
            } else {
                Ok(t)
            }
        })
        .unwrap_or_else(|e| e.into_compile_error().into())
}

fn extract_inner_type_of_option(ty: &syn::Type) -> Option<syn::Type> {
    if let syn::Type::Path(ref p) = ty {
        let first_segment = &p.path.segments[0];

        // `Option` type has only one segment with and ident `Option`
        if p.path.segments.len() != 1 || first_segment.ident != "Option" {
            return None;
        }

        // PathArgument of an `Option` is always `AngleBracketed`
        if let syn::PathArguments::AngleBracketed(ref angle_bracketed_type) =
            first_segment.arguments
        {
            // `Option` has only one argument inside angle brackets
            if angle_bracketed_type.args.len() != 1 {
                return None;
            }

            if let Some(syn::GenericArgument::Type(t)) = angle_bracketed_type.args.first() {
                return Some(t.clone());
            }
        }
    }
    None
}

fn contextual_field_type(ty: &syn::Type, context: &WPContextAttr) -> Result<syn::Type, syn::Error> {
    let mut ty = ty.clone();
    if let syn::Type::Path(ref mut p) = ty {
        assert!(p.path.segments.len() == 1);
        let segment: &mut syn::PathSegment = p.path.segments.first_mut().unwrap();
        let ident_name_without_prefix = ident_name_without_prefix(
            &segment.ident,
            ERROR_MISSING_SPARSE_PREFIX_FROM_WP_CONTEXTUAL_FIELD,
        )?;
        segment.ident = Ident::new(
            &ident_name_for_context(&ident_name_without_prefix, context),
            segment.ident.span(),
        );
        Ok(ty)
    } else {
        unimplemented!("Only syn::Meta::Path type is implemented for WPContextualField")
    }
}

fn ident_name_without_prefix(ident: &Ident, error_message: &str) -> Result<String, syn::Error> {
    let ident_name = ident.to_string();
    let incorrect_ident_error = syn::Error::new(ident.span(), error_message);
    let ident_name_without_prefix = ident_name
        .strip_prefix(IDENT_PREFIX)
        .ok_or_else(|| incorrect_ident_error.clone())?;
    syn::parse_str::<Ident>(ident_name_without_prefix)
        .map_err(|_| incorrect_ident_error.clone())?;
    Ok(ident_name_without_prefix.to_string())
}

fn ident_name_for_context(ident_name_without_prefix: &String, context: &WPContextAttr) -> String {
    format!("{}With{}Context", ident_name_without_prefix, context)
}

fn is_wp_context_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContext")
}

fn is_wp_contextual_field_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContextualField")
}

#[derive(Debug, PartialEq, Eq)]
struct WPParsedField {
    field: syn::Field,
    parsed_attrs: Vec<WPParsedAttr>,
}

#[derive(Debug, PartialEq, Eq)]
enum WPParsedAttr {
    ParsedWPContextualField,
    ParsedWPContext { contexts: Vec<WPContextAttr> },
    ExternalAttr { attr: syn::Attribute },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum WPContextAttr {
    Edit,
    Embed,
    View,
}

impl WPContextAttr {
    pub fn iter() -> Iter<'static, WPContextAttr> {
        [
            WPContextAttr::Edit,
            WPContextAttr::Embed,
            WPContextAttr::View,
        ]
        .iter()
    }
}

impl Display for WPContextAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Edit => "Edit",
                Self::Embed => "Embed",
                Self::View => "View",
            }
        )
    }
}

impl FromStr for WPContextAttr {
    type Err = WPDeriveParseAttrErrorType;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "edit" => Ok(Self::Edit),
            "embed" => Ok(Self::Embed),
            "view" => Ok(Self::View),
            _ => Err(WPDeriveParseAttrErrorType::UnexpectedWPContextLiteral {
                input: input.to_string(),
            }),
        }
    }
}

fn parse_field_attrs<'a>(
    fields: impl Iterator<Item = &'a syn::Field>,
) -> Result<Vec<WPParsedField>, WPDeriveParseAttrError> {
    fields
        .map(|f| {
            let parsed_attrs = f.attrs.iter().map(|attr| {
                if attr.path().segments.len() != 1 {
                    return Err(WPDeriveParseAttrError::unexpected_segment_count(attr.path().span()));
                }
                let path_segment = attr.path().segments.first().expect("There should be only 1 segment as validated previously using UnexpectedAttrPathSegmentCount error");
                let segment_ident = &path_segment.ident;
                if is_wp_contextual_field_ident(segment_ident) {
                    return Ok(WPParsedAttr::ParsedWPContextualField);
                }
                if is_wp_context_ident(segment_ident) {
                    if let syn::Meta::List(meta_list) = &attr.meta {
                        let contexts: Vec<WPContextAttr> = meta_list.tokens.clone().into_iter().filter_map(|t| {
                            if let proc_macro2::TokenTree::Ident(l) = t {
                                Some(WPContextAttr::from_str(&l.to_string()).map_err(|err_type| {
                                    WPDeriveParseAttrError::new(err_type, l.span())
                                }))
                            } else {
                                None
                            }
                        }).collect::<Result<Vec<WPContextAttr>, WPDeriveParseAttrError>>()?;
                        Ok(WPParsedAttr::ParsedWPContext { contexts })
                    } else {
                        Err(WPDeriveParseAttrError::unexpected_wp_context_meta(attr.meta.span()))
                    }
                } else {
                    Ok(WPParsedAttr::ExternalAttr { attr:attr.clone() })
                }
            }).collect::<Result<Vec<WPParsedAttr>, WPDeriveParseAttrError>>()?;
            Ok(WPParsedField {
                field: f.clone(),
                parsed_attrs
            })
        })
        .collect::<Result<Vec<WPParsedField>, WPDeriveParseAttrError>>()
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
enum WPDeriveParseAttrErrorType {
    #[error("Expected 'edit', 'embed' or 'view', found '{}'", input)]
    UnexpectedWPContextLiteral { input: String },
    // syn::Meta::Path or syn::Meta::NameValue
    #[error("Expected #[WPContext(edit, embed, view)]. Did you forget to add context types?")]
    UnexpectedWPContextMeta,
    #[error("UnexpectedAttrPathSegmentCount")]
    UnexpectedAttrPathSegmentCount,
}

struct WPDeriveParseAttrError {
    error_type: WPDeriveParseAttrErrorType,
    span: proc_macro2::Span,
}

impl WPDeriveParseAttrError {
    fn new(error_type: WPDeriveParseAttrErrorType, span: proc_macro2::Span) -> Self {
        Self { error_type, span }
    }

    fn unexpected_segment_count(span: proc_macro2::Span) -> Self {
        Self::new(
            WPDeriveParseAttrErrorType::UnexpectedAttrPathSegmentCount,
            span,
        )
    }

    fn unexpected_wp_context_meta(span: proc_macro2::Span) -> Self {
        Self::new(WPDeriveParseAttrErrorType::UnexpectedWPContextMeta, span)
    }
}

impl From<WPDeriveParseAttrError> for syn::Error {
    fn from(err: WPDeriveParseAttrError) -> Self {
        syn::Error::new(err.span, err.error_type.to_string())
    }
}
