use std::{fmt::Display, slice::Iter, str::FromStr};

use proc_macro::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, DeriveInput, Ident};

const IDENT_PREFIX: &str = "Sparse";

pub fn wp_contextual(ast: DeriveInput) -> Result<TokenStream, syn::Error> {
    let original_ident = &ast.ident;
    let original_ident_name = original_ident.to_string();

    let ident_name_without_prefix = original_ident_name.strip_prefix(IDENT_PREFIX).ok_or(
        WPContextualParseError::WPContextualMissingSparsePrefix
            .into_syn_error(original_ident.span()),
    )?;
    let fields =
        struct_fields(&ast.data).map_err(|err| err.into_syn_error(original_ident.span()))?;
    let parsed_fields = parse_fields(fields)?;

    let contextual_token_streams = WPContextAttr::iter().map(|current_context| {
        let cname = ident_name_for_context(ident_name_without_prefix, current_context);
        let cident = Ident::new(&cname, original_ident.span());
        let cfields = generate_context_fields(&parsed_fields, current_context)?;
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
}

// Validate that the given `data` is a `syn::Data::Struct` and extracts the fields from it
fn struct_fields(
    data: &syn::Data,
) -> Result<&syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, WPContextualParseError> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = data
    {
        Ok(named)
    } else {
        Err(WPContextualParseError::WPContextualNotAStruct)
    }
}

// Turns a list of `syn::Field`s to a list of `WPParsedField`s by parsing its attributes.
//
// The following errors are directly handled by this function:
// * `WPContextualParseAttrError::UnexpectedAttrPathSegmentCount`: The attribute path has multiple
// segments, separated by `::`. This case doesn't seem to be a valid syntax regardless of how
// we handle it, but it's handled as an error just in case.
// * `WPContextualParseAttrError::MissingWPContextMeta`: #[WPContext] attribute doesn't have any
// contexts.
// * `WPContextualParseError::WPContextualFieldWithoutWPContext`: #[WPContextualField] is added to
// a field that doesn't have the #[WPContext] attribute.
//
// It'll also handle incorrectly formatted #[WPContext] attribute through
// `parse_contexts_from_tokens` helper.
fn parse_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<Vec<WPParsedField>, syn::Error> {
    let parsed_fields = fields
        .iter()
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
                    if is_wp_contextual_option_ident(segment_ident) {
                        return Ok(WPParsedAttr::ParsedWPContextualOption);
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
        .collect::<Result<Vec<WPParsedField>, syn::Error>>()?;

    // Check if there are any fields that has #[WPContextualField] attribute,
    // but not the #[WPContext] attribute
    if let Some(pf) = parsed_fields
        .iter()
        .filter(|pf| {
            pf.parsed_attrs
                .contains(&WPParsedAttr::ParsedWPContextualField)
        })
        .find(|pf| {
            !pf.parsed_attrs.iter().any(|pf| match pf {
                WPParsedAttr::ParsedWPContext { contexts } => !contexts.is_empty(),
                _ => false,
            })
        })
    {
        return Err(WPContextualParseError::WPContextualFieldWithoutWPContext
            .into_syn_error(pf.field.span()));
    };
    // Check if there are any fields that has both #[WPContextualField] & #[WPContextualOption]
    // attributes and return an error. These attributes are incompatible with each other because
    // #[WPContextualOption] will leave the type as is, whereas #[WPContextualField] will modify it
    if let Some(pf) = parsed_fields.iter().find(|pf| {
        pf.parsed_attrs
            .contains(&WPParsedAttr::ParsedWPContextualField)
            && pf
                .parsed_attrs
                .contains(&WPParsedAttr::ParsedWPContextualOption)
    }) {
        return Err(
            WPContextualParseError::WPContextualBothOptionAndField.into_syn_error(pf.field.span())
        );
    }

    Ok(parsed_fields)
}

// Generates fields for the given context.
//
// It'll filter out any fields that don't have the given context, handle any mappings due to
// #[WPContextualField] attribute and remove #[WPContext] and #[WPContextualField] attributes.
fn generate_context_fields(
    parsed_fields_attrs: &[WPParsedField],
    context: &WPContextAttr,
) -> Result<Vec<syn::Field>, syn::Error> {
    parsed_fields_attrs
        .iter()
        .filter(|pf| {
            // Filter out any field that doesn't have this context
            pf.parsed_attrs.iter().any(|parsed_attr| {
                if let WPParsedAttr::ParsedWPContext { contexts } = parsed_attr {
                    contexts.iter().any(|c| c == context)
                } else {
                    false
                }
            })
        })
        .map(|pf| {
            let f = &pf.field;

            let new_type = if pf
                .parsed_attrs
                .contains(&WPParsedAttr::ParsedWPContextualOption)
            {
                f.ty.clone()
            } else {
                let mut new_type = extract_inner_type_of_option(&f.ty).unwrap_or(f.ty.clone());
                if f.attrs.iter().any(|attr| {
                    attr.path()
                        .segments
                        .iter()
                        .any(|s| is_wp_contextual_field_ident(&s.ident))
                }) {
                    // If the field has #[WPContextualField] attr, map it to its contextual field type
                    new_type = contextual_field_type(&new_type, context)?;
                }
                new_type
            };
            Ok::<syn::Field, syn::Error>(syn::Field {
                // Remove the WPContext & WPContextualField attributes from the generated field
                attrs: pf
                    .parsed_attrs
                    .iter()
                    .filter_map(|parsed_attr| {
                        // The generated field should only contain external attributes
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
        .collect()
}

// Returns a contextual type for the given type.
//
// ```
// #[derive(WPContextual)]
// pub struct SparseFoo {
//     #[WPContext(edit)]
//     #[WPContextualField]
//     pub bar: Option<SparseBar>,
// }
//
// #[WPContextual]
// pub struct SparseBar {
//     #[WPContext(edit)]
//     pub baz: Option<u32>,
// }
// ```
//
// Given the above, we'd like to generate:
//
// ```
// pub struct FooWithEditContext {
//     pub bar: BarWithEditContext,
// }
//
// pub struct BarWithEditContext {
//     pub baz: u32,
// }
// ```
//
// In this case, this function takes the `Option<SparseBar>` type and `&WPContextAttr::Edit`
// and turns it into `BarWithEditContext` type.
fn contextual_field_type(ty: &syn::Type, context: &WPContextAttr) -> Result<syn::Type, syn::Error> {
    let mut ty = ty.clone();
    let inner_segment = find_contextual_field_inner_segment(&mut ty)?;
    let ident_name_without_prefix = match inner_segment.ident.to_string().strip_prefix(IDENT_PREFIX)
    {
        Some(ident) => Ok(ident.to_string()),
        None => Err(WPContextualParseError::WPContextualFieldMissingSparsePrefix
            .into_syn_error(inner_segment.ident.span())),
    }?;
    inner_segment.ident = Ident::new(
        &ident_name_for_context(&ident_name_without_prefix, context),
        inner_segment.ident.span(),
    );
    Ok(ty)
}

// This is a recursive function that finds the inner path segment of a #[WPContextualField].
//
// There are many cases that are not supported by #[WPContextualField] mainly because these cases
// are also not supported by UniFFI and/or serde. For example, we can't use tuples, references,
// functions etc as a #[WPContextualField], but we also wouldn't expect to.
//
// The main cases we need to handle are Option<SparseFoo> and Option<Vec<SparseFoo>> types where
// the API is either returning a single object or a list of objects.
//
// By making this a recursive function, we also handle cases such as Option<Vec<Vec<SparseFoo>>>,
// but it's very unlikely that this case will ever be used.
// We also support multiple segments, such as Option<std::vec::Vec<SparseFoo>>.
//
// The error returned from this function is a generic one, pointing out that
// we don't support the given type. In this case, this is our best option because
// building an exhaustive list of errors is neither feasible nor useful.
fn find_contextual_field_inner_segment(
    ty: &mut syn::Type,
) -> Result<&mut syn::PathSegment, syn::Error> {
    let unsupported_err =
        WPContextualParseError::WPContextualFieldTypeNotSupported.into_syn_error(ty.span());
    if let syn::Type::Path(ref mut p) = ty {
        // A `syn::Type::Path` has to have at least one segment.
        assert!(!p.path.segments.is_empty());

        // If it has multiple segments, we are only interested in modifying the last one.
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
        let segment: &mut syn::PathSegment = p.path.segments.last_mut().unwrap();

        match segment.arguments {
            // No inner type
            //
            // ```
            // #[derive(WPContextual)]
            // pub struct SparseFoo {
            //     #[WPContext(edit)]
            //     #[WPContextualField]
            //     pub bar: Option<SparseBar>,
            // }
            // ```
            syn::PathArguments::None => Ok(segment),
            // Type is surrounded with angled brackets
            //
            // ```
            // #[derive(WPContextual)]
            // pub struct SparseFoo {
            //     #[WPContext(edit)]
            //     #[WPContextualField]
            //     pub bar: Option<Vec<SparseBar>>,
            //     #[WPContext(view)]
            //     #[WPContextualField]
            //     pub baz: Option<Vec<Vec<SparseBaz>>>,
            // }
            // ```
            syn::PathArguments::AngleBracketed(ref mut path_args) => path_args
                .args
                .iter_mut()
                .find_map(|generic_arg| {
                    if let syn::GenericArgument::Type(tty) = generic_arg {
                        Some(find_contextual_field_inner_segment(tty))
                    } else {
                        None
                    }
                })
                .ok_or(unsupported_err)?,
            syn::PathArguments::Parenthesized(_) => Err(unsupported_err),
        }
    } else {
        Err(unsupported_err)
    }
}

// Extracts `Foo` from `Option<Foo>`.
//
// It currently doesn't support `std::option::Option<Foo>` or `core::option::Option<Foo>`. Although
// it'd be fairly straightforward to do so, it's also unnecessary as we want to encourage the
// usage of simple `Option` type for consistency.
fn extract_inner_type_of_option(ty: &syn::Type) -> Option<syn::Type> {
    if let syn::Type::Path(ref p) = ty {
        let first_segment = &p.path.segments[0];

        // `Option` type has only one segment with an ident `Option`
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

fn ident_name_for_context(ident_name_without_prefix: &str, context: &WPContextAttr) -> String {
    format!("{}With{}Context", ident_name_without_prefix, context)
}

fn is_wp_context_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContext")
}

fn is_wp_contextual_field_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContextualField")
}

fn is_wp_contextual_option_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContextualOption")
}

// ```
// #[WPContextual]
// pub struct SparseFoo {
//     #[WPContext(edit, embed)]
//     pub bar: Option<u32>,
// }
// ```
//
// In this example, given the `TokenStream` for `edit, embed`, turns it into
// vec![WPContextAttr::Edit, WPContextAttr::Embed].
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
            proc_macro2::TokenTree::Group(g) => Some(Err(
                WPContextualParseAttrError::UnexpectedToken.into_syn_error(g.span()),
            )),
            proc_macro2::TokenTree::Literal(l) => Some(Err(
                WPContextualParseAttrError::UnexpectedLiteralToken.into_syn_error(l.span()),
            )),
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
    ParsedWPContextualOption,
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
            _ => Err(WPContextualParseAttrError::UnexpectedWPContextIdent {
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
    #[error("#[WPContextualField] & #[WPContextualOption] can't be used together")]
    WPContextualBothOptionAndField,
    #[error(
        "WPContextualField field types need to start with '{}' prefix",
        IDENT_PREFIX
    )]
    WPContextualFieldMissingSparsePrefix,
    #[error("#[WPContextualField] doesn't have any contexts. Did you forget to add #[WPContext] attribute?")]
    WPContextualFieldWithoutWPContext,
    #[error("Only Option<SparseFoo> & Option<Vec<SparseFoo>> types are supported by #[WPContextualField]")]
    WPContextualFieldTypeNotSupported,
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
    // It's possible to trigger this error by using something like `#[wp_contextual::WPContext]`,
    // however that's not a valid syntax. There is probably no valid syntax that uses `::` in the
    // current setup, but in case we are missing anything, we should be able to improve the
    // messaging by asking it to be reported.
    #[error("Expected #[WPContext] or #[WPContextualField], found multi-segment path.\nPlease report this case to the `wp_contextual` developers.")]
    UnexpectedAttrPathSegmentCount,
    #[error("Did you mean ','?")]
    UnexpectedPunct,
    #[error("Expected 'edit', 'embed' or 'view', found '{}'", input)]
    UnexpectedWPContextIdent { input: String },
    // syn::Meta::Path or syn::Meta::NameValue
    #[error("Expected #[WPContext(edit, embed, view)]. Did you forget to add context types?")]
    MissingWPContextMeta,
    #[error("Expected #[WPContext(edit, embed, view)], found unsupported tokens")]
    UnexpectedToken,
    #[error("Expected #[WPContext(edit, embed, view)]. Try removing the quotation?")]
    UnexpectedLiteralToken,
}

impl WPContextualParseAttrError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn find_contextual_field_inner_segment_simple() {
        validate_find_contextual_field_inner_segment(
            "Bar",
            parse_quote! {
                let foo: Option<Bar>;
            },
        );
    }

    #[test]
    fn find_contextual_field_inner_segment_wrapped_in_vec() {
        validate_find_contextual_field_inner_segment(
            "Bar",
            parse_quote! {
                let foo: Option<Vec<Bar>>;
            },
        );
    }

    #[test]
    fn find_contextual_field_inner_segment_wrapped_in_segmented_vec() {
        validate_find_contextual_field_inner_segment(
            "Bar",
            parse_quote! {
                let foo: Option<std::vec::Vec<Bar>>;
            },
        );
    }

    #[test]
    fn find_contextual_field_inner_segment_wrapped_in_multiple_vecs() {
        validate_find_contextual_field_inner_segment(
            "Bar",
            parse_quote! {
                let foo: Option<std::vec::Vec<Vec<Bar>>>;
            },
        );
    }

    #[test]
    fn find_contextual_field_inner_segment_error_tuple_not_supported() {
        let mut input_type = type_from_simple_let_stmt(parse_quote! {
            let foo: (u32, u32);
        });
        assert_eq!(
            find_contextual_field_inner_segment(&mut input_type)
                .unwrap_err()
                .to_string(),
            WPContextualParseError::WPContextualFieldTypeNotSupported.to_string()
        );
    }

    #[test]
    fn extract_inner_type_of_option_simple() {
        let input_type = type_from_simple_let_stmt(parse_quote! {
            let foo: Option<Foo>;
        });
        let expected_type = type_from_simple_let_stmt(parse_quote! {
            let foo: Foo;
        });
        assert_eq!(
            extract_inner_type_of_option(&input_type),
            Some(expected_type)
        );
    }

    fn validate_find_contextual_field_inner_segment(result: &str, stmt: syn::Stmt) {
        let mut input_type = type_from_simple_let_stmt(stmt);
        assert_eq!(
            find_contextual_field_inner_segment(&mut input_type)
                .unwrap()
                .ident
                .to_string(),
            result
        );
    }

    fn type_from_simple_let_stmt(stmt: syn::Stmt) -> syn::Type {
        if let syn::Stmt::Local(syn::Local {
            pat: syn::Pat::Type(syn::PatType { ty, .. }),
            ..
        }) = stmt
        {
            Some(*ty)
        } else {
            None
        }
        .unwrap()
    }
}
