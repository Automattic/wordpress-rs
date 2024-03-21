use std::{fmt::Display, slice::Iter, str::FromStr};

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Ident};

const IDENT_PREFIX: &str = "Sparse";

#[proc_macro_derive(WPContextual, attributes(WPContext, WPContextualField))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);
    let original_ident = &ast.ident;
    let origina_ident_name = original_ident.to_string();
    let ident_name_without_prefix = match origina_ident_name.strip_prefix(IDENT_PREFIX) {
        Some(ident) => ident,
        None => {
            return WPContextualParseError::WPContextualMissingSparsePrefix
                .into_syn_error(original_ident.span())
                .into_compile_error()
                .into()
        }
    };
    let fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma> =
        if let syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
            ..
        }) = ast.data
        {
            named
        } else {
            return WPContextualParseError::WPContextualNotAStruct
                .into_syn_error(original_ident.span())
                .into_compile_error()
                .into();
        };

    let parsed_fields = match parse_field_attrs(fields.iter()) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
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
        return WPContextualParseError::WPContextualFieldWithoutWPContext
            .into_syn_error(pf.field.span())
            .to_compile_error()
            .into();
    };

    let contextual_token_streams = WPContextAttr::iter().map(|context_attr| {
        let cname = ident_name_for_context(ident_name_without_prefix, context_attr);
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
                Err(WPContextualParseError::EmptyResult.into_syn_error(original_ident.span()))
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
        // A `syn::Type::Path` has to have at least one segment. If it has multiple segments, we are only
        // interested in modifying the last one.
        //
        // Consider the following:
        // ```
        // #[derive(WPContextual)]
        // pub struct SparseFoo {
        //     #[WPContext(edit)]
        //     #[WPContextualField]
        //     pub bar: Option<SparseBar>,
        //     #[WPContext(view)]
        //     #[WPContextualField]
        //     pub baz: Option<baz::SparseBaz>,
        // }
        // ```
        //
        // `SparseBar` only has one segment and `baz::SparseBaz` has two segments. In each case,
        // we want to get the last segment, drop the `Sparse` prefix and attach the `With{}Context`
        // postfix to it, depending on the context. In this case, the resulting generated code
        // should look like the following:
        //
        // pub struct FooWithEditContext {
        //     pub bar: BarWithEditContext,
        // }
        //
        // pub struct FooWithViewContext {
        //     pub baz: baz::BazWithEditContext,
        // }
        // ```
        //
        assert!(!p.path.segments.is_empty());
        let segment: &mut syn::PathSegment = p.path.segments.last_mut().unwrap();

        let ident_name_without_prefix = match segment.ident.to_string().strip_prefix(IDENT_PREFIX) {
            Some(ident) => Ok(ident.to_string()),
            None => Err(WPContextualParseError::WPContextualFieldMissingSparsePrefix
                .into_syn_error(segment.ident.span())),
        }?;
        segment.ident = Ident::new(
            &ident_name_for_context(&ident_name_without_prefix, context),
            segment.ident.span(),
        );
        Ok(ty)
    } else {
        unimplemented!("Only syn::Type::Path variant is implemented for WPContextualField")
    }
}

fn ident_name_for_context(ident_name_without_prefix: &str, context: &WPContextAttr) -> String {
    format!("{}With{}Context", ident_name_without_prefix, context)
}

fn is_wp_context_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContext")
}

fn is_wp_contextual_field_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContextualField")
}

fn parse_field_attrs<'a>(
    fields: impl Iterator<Item = &'a syn::Field>,
) -> Result<Vec<WPParsedField>, syn::Error> {
    fields
        .map(|f| {
            let parsed_attrs = f
                .attrs
                .iter()
                .map(|attr| {
                    if attr.path().segments.len() != 1 {
                        return Err(WPContextualParseAttrError::UnexpectedAttrPathSegmentCount
                            .into_syn_error(attr.path().span()));
                    }
                    let path_segment = attr
                        .path()
                        .segments
                        .first()
                        .expect("Already validated that there is only one segment");
                    let segment_ident = &path_segment.ident;
                    if is_wp_contextual_field_ident(segment_ident) {
                        return Ok(WPParsedAttr::ParsedWPContextualField);
                    }
                    if is_wp_context_ident(segment_ident) {
                        if let syn::Meta::List(meta_list) = &attr.meta {
                            let contexts = parse_contexts_from_tokens(meta_list.tokens.clone())?;
                            Ok(WPParsedAttr::ParsedWPContext { contexts })
                        } else {
                            Err(WPContextualParseAttrError::MissingWPContextMeta
                                .into_syn_error(attr.meta.span()))
                        }
                    } else {
                        Ok(WPParsedAttr::ExternalAttr { attr: attr.clone() })
                    }
                })
                .collect::<Result<Vec<WPParsedAttr>, syn::Error>>()?;
            Ok(WPParsedField {
                field: f.clone(),
                parsed_attrs,
            })
        })
        .collect::<Result<Vec<WPParsedField>, syn::Error>>()
}

fn parse_contexts_from_tokens(
    tokens: proc_macro2::TokenStream,
) -> Result<Vec<WPContextAttr>, syn::Error> {
    tokens
        .into_iter()
        .filter_map(|t| match t {
            proc_macro2::TokenTree::Ident(ident) => Some(
                WPContextAttr::from_str(&ident.to_string())
                    .map_err(|error_type| error_type.into_syn_error(ident.span())),
            ),
            proc_macro2::TokenTree::Punct(p) => {
                if p.as_char() == ',' {
                    None
                } else {
                    Some(Err(
                        WPContextualParseAttrError::UnexpectedPunct.into_syn_error(p.span())
                    ))
                }
            }
            _ => None,
        })
        .collect::<Result<Vec<WPContextAttr>, syn::Error>>()
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
    type Err = WPContextualParseAttrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "edit" => Ok(Self::Edit),
            "embed" => Ok(Self::Embed),
            "view" => Ok(Self::View),
            _ => Err(WPContextualParseAttrError::UnexpectedWPContextLiteral {
                input: input.to_string(),
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WPContextualParseError {
    #[error(
        "WPContextual didn't generate anything. Did you forget to add #[WPContext] attribute?"
    )]
    EmptyResult,
    #[error(
        "WPContextualField field types need to start with '{}' prefix",
        IDENT_PREFIX
    )]
    WPContextualFieldMissingSparsePrefix,
    #[error("#[WPContextualField] doesn't have any contexts. Did you forget to add #[WPContext] attribute?")]
    WPContextualFieldWithoutWPContext,
    #[error("WPContextual types need to start with '{}' prefix. This prefix will be removed from the generated Structs, so it needs to be followed up with a proper Rust type name, starting with an uppercase letter.", IDENT_PREFIX)]
    WPContextualMissingSparsePrefix,
    #[error("#[WPContextual] is only implemented for Structs")]
    WPContextualNotAStruct,
}

impl WPContextualParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
enum WPContextualParseAttrError {
    // It's possible to trigger this error by using something like `#[wp_derive::WPContext]`,
    // however that's not a valid syntax. There is probably no valid syntax that uses `::` in the
    // current setup, but in case we are missing anything, we should be able to improve the
    // messaging by asking it to be reported.
    #[error("Expected #[WPContext] or #[WPContextualField], found multi-segment path.\nPlease report to the `wp_derive` developers how you triggered this error type so that a test for it can be added.")]
    UnexpectedAttrPathSegmentCount,
    #[error("Did you mean ','?")]
    UnexpectedPunct,
    #[error("Expected 'edit', 'embed' or 'view', found '{}'", input)]
    UnexpectedWPContextLiteral { input: String },
    // syn::Meta::Path or syn::Meta::NameValue
    #[error("Expected #[WPContext(edit, embed, view)]. Did you forget to add context types?")]
    MissingWPContextMeta,
}

impl WPContextualParseAttrError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
