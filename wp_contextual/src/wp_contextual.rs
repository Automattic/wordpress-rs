use std::{fmt::Display, slice::Iter, str::FromStr};

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, DeriveInput, Field, Ident};

const IDENT_PREFIX: &str = "Sparse";

pub fn wp_contextual(ast: DeriveInput) -> Result<TokenStream, syn::Error> {
    let original_ident = &ast.ident;
    let original_ident_name = original_ident.to_string();

    let ident_name_without_prefix = original_ident_name.strip_prefix(IDENT_PREFIX).ok_or(
        WpContextualParseError::WpContextualMissingSparsePrefix
            .into_syn_error(original_ident.span()),
    )?;
    let fields =
        struct_fields(&ast.data).map_err(|err| err.into_syn_error(original_ident.span()))?;
    let parsed_fields = parse_fields(fields)?;

    let contextual_token_streams = WpContextAttr::iter().map(|current_context| {
        let generate_type = |ident, generated_fields: Vec<Field>| {
            if !generated_fields.is_empty() {
                quote! {
                    #[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record)]
                    pub struct #ident {
                        #(#generated_fields,)*
                    }
                }
                .into()
            } else {
                TokenStream::new()
            }
        };
        let non_sparse_type_fields =
            generate_context_fields(&parsed_fields, current_context, true)?;
        let sparse_type_fields = generate_context_fields(&parsed_fields, current_context, false)?;

        let non_sparse_ident = Ident::new(
            &ident_name_for_context(ident_name_without_prefix, current_context),
            original_ident.span(),
        );
        let sparse_field_ident = Ident::new(
            &ident_name_for_context(
                format!("{}Field", original_ident_name).as_str(),
                current_context,
            ),
            original_ident.span(),
        );
        let sparse_ident = Ident::new(
            &ident_name_for_context(original_ident_name.as_str(), current_context),
            original_ident.span(),
        );
        let non_sparse_type = generate_type(non_sparse_ident, non_sparse_type_fields);
        let sparse_field_type = generate_sparse_field_type(sparse_field_ident, &sparse_type_fields);
        let sparse_type = generate_type(sparse_ident, sparse_type_fields);
        Ok(TokenStream::from_iter([
            non_sparse_type,
            sparse_type,
            sparse_field_type,
        ]))
    });
    contextual_token_streams
        .collect::<Result<Vec<TokenStream>, syn::Error>>()
        .map(TokenStream::from_iter)
        .and_then(|t: TokenStream| {
            if t.is_empty() {
                Err(WpContextualParseError::EmptyResult.into_syn_error(original_ident.span()))
            } else {
                Ok(t)
            }
        })
}

// Validate that the given `data` is a `syn::Data::Struct` and extracts the fields from it
fn struct_fields(
    data: &syn::Data,
) -> Result<&syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, WpContextualParseError> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = data
    {
        Ok(named)
    } else {
        Err(WpContextualParseError::WpContextualNotAStruct)
    }
}

// Turns a list of `syn::Field`s to a list of `WpParsedField`s by parsing its attributes.
//
// The following errors are directly handled by this function:
// * `WpContextualParseAttrError::UnexpectedAttrPathSegmentCount`: The attribute path has multiple
// segments, separated by `::`. This case doesn't seem to be a valid syntax regardless of how
// we handle it, but it's handled as an error just in case.
// * `WpContextualParseAttrError::MissingWpContextMeta`: #[WpContext] attribute doesn't have any
// contexts.
// * `WpContextualParseError::WpContextualFieldWithoutWpContext`: #[WpContextualField] is added to
// a field that doesn't have the #[WpContext] attribute.
// * `WpContextualParseError::WpContextualOptionWithoutWpContext`: #[WpContextualOption] is added to
// a field that doesn't have the #[WpContext] attribute.
// * `WpContextualParseError::WpContextualBothOptionAndField`: #[WpContextualField] and
// #[WpContextualOption] attributes were used together.
//
// It'll also handle incorrectly formatted #[WpContext] attribute through
// `parse_contexts_from_tokens` helper.
fn parse_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<Vec<WpParsedField>, syn::Error> {
    let parsed_fields = fields
        .iter()
        .map(|f| {
            let parsed_attrs = f
                .attrs
                .iter()
                .map(|attr| {
                    if attr.path().segments.len() != 1 {
                        return Err(WpContextualParseAttrError::UnexpectedAttrPathSegmentCount
                            .into_syn_error(attr.path().span()));
                    }
                    let path_segment = attr
                        .path()
                        .segments
                        .first()
                        .expect("Already validated that there is only one segment");
                    let segment_ident = &path_segment.ident;
                    if is_wp_contextual_field_ident(segment_ident) {
                        return Ok(WpParsedAttr::ParsedWpContextualField);
                    }
                    if is_wp_contextual_option_ident(segment_ident) {
                        return Ok(WpParsedAttr::ParsedWpContextualOption);
                    }
                    if is_wp_context_ident(segment_ident) {
                        if let syn::Meta::List(meta_list) = &attr.meta {
                            let contexts = parse_contexts_from_tokens(meta_list.tokens.clone())?;
                            Ok(WpParsedAttr::ParsedWpContext { contexts })
                        } else {
                            Err(WpContextualParseAttrError::MissingWpContextMeta
                                .into_syn_error(attr.meta.span()))
                        }
                    } else {
                        Ok(WpParsedAttr::ExternalAttr { attr: attr.clone() })
                    }
                })
                .collect::<Result<Vec<WpParsedAttr>, syn::Error>>()?;
            Ok(WpParsedField {
                field: f.clone(),
                parsed_attrs,
            })
        })
        .collect::<Result<Vec<WpParsedField>, syn::Error>>()?;

    let assert_has_wp_context_attribute_if_it_has_given_attribute =
        |attribute_to_check: WpParsedAttr, error_type: WpContextualParseError| {
            if let Some(pf) = parsed_fields
                .iter()
                .filter(|pf| pf.parsed_attrs.contains(&attribute_to_check))
                .find(|pf| {
                    !pf.parsed_attrs.iter().any(|pf| match pf {
                        WpParsedAttr::ParsedWpContext { contexts } => !contexts.is_empty(),
                        _ => false,
                    })
                })
            {
                Err(error_type.into_syn_error(pf.field.span()))
            } else {
                Ok(())
            }
        };

    // Check if there are any fields that has #[WpContextualField] attribute,
    // but not the #[WpContext] attribute
    assert_has_wp_context_attribute_if_it_has_given_attribute(
        WpParsedAttr::ParsedWpContextualField,
        WpContextualParseError::WpContextualFieldWithoutWpContext,
    )?;

    // Check if there are any fields that has #[WpContextualField] attribute,
    // but not the #[WpContext] attribute
    assert_has_wp_context_attribute_if_it_has_given_attribute(
        WpParsedAttr::ParsedWpContextualOption,
        WpContextualParseError::WpContextualOptionWithoutWpContext,
    )?;

    // Check if there are any fields that has both #[WpContextualField] & #[WpContextualOption]
    // attributes and return an error. These attributes are incompatible with each other because
    // #[WpContextualOption] will leave the type as is, whereas #[WpContextualField] will modify
    // it.
    if let Some(pf) = parsed_fields.iter().find(|pf| {
        pf.parsed_attrs
            .contains(&WpParsedAttr::ParsedWpContextualField)
            && pf
                .parsed_attrs
                .contains(&WpParsedAttr::ParsedWpContextualOption)
    }) {
        return Err(
            WpContextualParseError::WpContextualBothOptionAndField.into_syn_error(pf.field.span())
        );
    }

    Ok(parsed_fields)
}

fn generate_sparse_field_type(type_ident: Ident, fields: &[Field]) -> TokenStream {
    let mut variant_idents = Vec::with_capacity(fields.len());
    let mut as_field_names = Vec::with_capacity(fields.len());
    for f in fields {
        if let Some(f_ident) = &f.ident {
            let field_name = f_ident.to_string();
            let variant_ident = format_ident!("{}", field_name.to_case(Case::UpperCamel));
            let field_name = field_name.as_str();

            variant_idents.push(variant_ident.clone());
            as_field_names.push(quote! {
                Self::#variant_ident => #field_name
            });
        }
    }
    if variant_idents.is_empty() {
        return TokenStream::new();
    }
    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
        pub enum #type_ident {
            #(#variant_idents,)*
        }
        impl #type_ident {
            pub fn as_field_name(&self) -> &str {
                match self {
                    #(#as_field_names,)*
                }
            }
        }
    }
    .into()
}

// Generates fields for the given context.
//
// It'll filter out any fields that don't have the given context, handle any mappings due to
// #[WpContextualField] attribute and remove #[WpContext] and #[WpContextualField] attributes.
fn generate_context_fields(
    parsed_fields_attrs: &[WpParsedField],
    context: &WpContextAttr,
    should_extract_option: bool,
) -> Result<Vec<syn::Field>, syn::Error> {
    parsed_fields_attrs
        .iter()
        .filter(|pf| {
            // Filter out any field that doesn't have this context
            pf.parsed_attrs.iter().any(|parsed_attr| {
                if let WpParsedAttr::ParsedWpContext { contexts } = parsed_attr {
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
                .contains(&WpParsedAttr::ParsedWpContextualOption)
            {
                f.ty.clone()
            } else {
                let mut new_type = if should_extract_option {
                    extract_inner_type_of_option(&f.ty).unwrap_or(f.ty.clone())
                } else {
                    f.ty.clone()
                };
                if f.attrs.iter().any(|attr| {
                    attr.path()
                        .segments
                        .iter()
                        .any(|s| is_wp_contextual_field_ident(&s.ident))
                }) {
                    // If the field has #[WpContextualField] attr, map it to its contextual field type
                    new_type = if should_extract_option {
                        contextual_non_sparse_field_type(&new_type, context)?
                    } else {
                        contextual_sparse_field_type(&new_type, context)?
                    }
                }
                new_type
            };
            Ok::<syn::Field, syn::Error>(syn::Field {
                // Remove the WpContext & WpContextualField attributes from the generated field
                attrs: pf
                    .parsed_attrs
                    .iter()
                    .filter_map(|parsed_attr| {
                        // The generated field should only contain external attributes
                        if let WpParsedAttr::ExternalAttr { attr } = parsed_attr {
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

// Returns a contextual non-sparse type for the given type.
//
// ```
// #[derive(WpContextual)]
// pub struct SparseFoo {
//     #[WpContext(edit)]
//     #[WpContextualField]
//     pub bar: Option<SparseBar>,
// }
//
// #[WpContextual]
// pub struct SparseBar {
//     #[WpContext(edit)]
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
// In this case, this function takes the `Option<SparseBar>` type and `&WpContextAttr::Edit`
// and turns it into `BarWithEditContext` type.
fn contextual_non_sparse_field_type(
    ty: &syn::Type,
    context: &WpContextAttr,
) -> Result<syn::Type, syn::Error> {
    let mut ty = ty.clone();
    let inner_segment = find_contextual_field_inner_segment(&mut ty)?;
    let ident_name_without_prefix = match inner_segment.ident.to_string().strip_prefix(IDENT_PREFIX)
    {
        Some(ident) => Ok(ident.to_string()),
        None => Err(WpContextualParseError::WpContextualFieldMissingSparsePrefix
            .into_syn_error(inner_segment.ident.span())),
    }?;
    inner_segment.ident = Ident::new(
        &ident_name_for_context(&ident_name_without_prefix, context),
        inner_segment.ident.span(),
    );
    Ok(ty)
}

// Returns a contextual sparse type for the given type.
//
// ```
// #[derive(WpContextual)]
// pub struct SparseFoo {
//     #[WpContext(edit)]
//     #[WpContextualField]
//     pub bar: Option<SparseBar>,
// }
//
// #[WpContextual]
// pub struct SparseBar {
//     #[WpContext(edit)]
//     pub baz: Option<u32>,
// }
// ```
//
// Given the above, we'd like to generate:
//
// ```
// pub struct SparseFooWithEditContext {
//     pub bar: Option<SparseBarWithEditContext>,
// }
//
// pub struct SparseBarWithEditContext {
//     pub baz: Option<u32>,
// }
// ```
//
// In this case, this function takes the `Option<SparseBar>` type and `&WpContextAttr::Edit`
// and turns it into `Option<SparseBarWithEditContext>` type.
fn contextual_sparse_field_type(
    ty: &syn::Type,
    context: &WpContextAttr,
) -> Result<syn::Type, syn::Error> {
    let mut ty = ty.clone();
    let inner_segment = find_contextual_field_inner_segment(&mut ty)?;
    inner_segment.ident = Ident::new(
        &ident_name_for_context(inner_segment.ident.to_string().as_str(), context),
        inner_segment.ident.span(),
    );
    Ok(ty)
}

// This is a recursive function that finds the inner path segment of a #[WpContextualField].
//
// There are many cases that are not supported by #[WpContextualField] mainly because these cases
// are also not supported by UniFFI and/or serde. For example, we can't use tuples, references,
// functions etc as a #[WpContextualField], but we also wouldn't expect to.
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
        WpContextualParseError::WpContextualFieldTypeNotSupported.into_syn_error(ty.span());
    if let syn::Type::Path(ref mut p) = ty {
        // A `syn::Type::Path` has to have at least one segment.
        assert!(!p.path.segments.is_empty());

        // If it has multiple segments, we are only interested in modifying the last one.
        //
        // Consider the following:
        // ```
        // #[derive(WpContextual)]
        // pub struct SparseFoo {
        //     #[WpContext(edit)]
        //     #[WpContextualField]
        //     pub bar: Option<SparseBar>,
        //     #[WpContext(view)]
        //     #[WpContextualField]
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
            // #[derive(WpContextual)]
            // pub struct SparseFoo {
            //     #[WpContext(edit)]
            //     #[WpContextualField]
            //     pub bar: Option<SparseBar>,
            // }
            // ```
            syn::PathArguments::None => Ok(segment),
            // Type is surrounded with angled brackets
            //
            // ```
            // #[derive(WpContextual)]
            // pub struct SparseFoo {
            //     #[WpContext(edit)]
            //     #[WpContextualField]
            //     pub bar: Option<Vec<SparseBar>>,
            //     #[WpContext(view)]
            //     #[WpContextualField]
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

fn ident_name_for_context(ident_name_without_prefix: &str, context: &WpContextAttr) -> String {
    format!("{}With{}Context", ident_name_without_prefix, context)
}

fn is_wp_context_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WpContext")
}

fn is_wp_contextual_field_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WpContextualField")
}

fn is_wp_contextual_option_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WpContextualOption")
}

// ```
// #[WpContextual]
// pub struct SparseFoo {
//     #[WpContext(edit, embed)]
//     pub bar: Option<u32>,
// }
// ```
//
// In this example, given the `TokenStream` for `edit, embed`, turns it into
// vec![WpContextAttr::Edit, WpContextAttr::Embed].
fn parse_contexts_from_tokens(
    tokens: proc_macro2::TokenStream,
) -> Result<Vec<WpContextAttr>, syn::Error> {
    tokens
        .into_iter()
        .filter_map(|t| match t {
            proc_macro2::TokenTree::Ident(ident) => Some(
                WpContextAttr::from_str(&ident.to_string())
                    .map_err(|error_type| error_type.into_syn_error(ident.span())),
            ),
            proc_macro2::TokenTree::Punct(p) => {
                if p.as_char() == ',' {
                    None
                } else {
                    Some(Err(
                        WpContextualParseAttrError::UnexpectedPunct.into_syn_error(p.span())
                    ))
                }
            }
            proc_macro2::TokenTree::Group(g) => Some(Err(
                WpContextualParseAttrError::UnexpectedToken.into_syn_error(g.span()),
            )),
            proc_macro2::TokenTree::Literal(l) => Some(Err(
                WpContextualParseAttrError::UnexpectedLiteralToken.into_syn_error(l.span()),
            )),
        })
        .collect::<Result<Vec<WpContextAttr>, syn::Error>>()
}

#[derive(Debug, PartialEq, Eq)]
struct WpParsedField {
    field: syn::Field,
    parsed_attrs: Vec<WpParsedAttr>,
}

#[derive(Debug, PartialEq, Eq)]
enum WpParsedAttr {
    ParsedWpContextualField,
    ParsedWpContextualOption,
    ParsedWpContext { contexts: Vec<WpContextAttr> },
    ExternalAttr { attr: syn::Attribute },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum WpContextAttr {
    Edit,
    Embed,
    View,
}

impl WpContextAttr {
    pub fn iter() -> Iter<'static, WpContextAttr> {
        [
            WpContextAttr::Edit,
            WpContextAttr::Embed,
            WpContextAttr::View,
        ]
        .iter()
    }
}

impl Display for WpContextAttr {
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

impl FromStr for WpContextAttr {
    type Err = WpContextualParseAttrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "edit" => Ok(Self::Edit),
            "embed" => Ok(Self::Embed),
            "view" => Ok(Self::View),
            _ => Err(WpContextualParseAttrError::UnexpectedWpContextIdent {
                input: input.to_string(),
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WpContextualParseError {
    #[error(
        "WpContextual didn't generate anything. Did you forget to add #[WpContext] attribute?"
    )]
    EmptyResult,
    #[error("#[WpContextualField] & #[WpContextualOption] can't be used together")]
    WpContextualBothOptionAndField,
    #[error(
        "WpContextualField field types need to start with '{}' prefix",
        IDENT_PREFIX
    )]
    WpContextualFieldMissingSparsePrefix,
    #[error("#[WpContextualField] doesn't have any contexts. Did you forget to add #[WpContext] attribute?")]
    WpContextualFieldWithoutWpContext,
    #[error("Only Option<SparseFoo> & Option<Vec<SparseFoo>> types are supported by #[WpContextualField]")]
    WpContextualFieldTypeNotSupported,
    #[error("WpContextual types need to start with '{}' prefix. This prefix will be removed from the generated Structs, so it needs to be followed up with a proper Rust type name, starting with an uppercase letter.", IDENT_PREFIX)]
    WpContextualMissingSparsePrefix,
    #[error("#[WpContextual] is only implemented for Structs")]
    WpContextualNotAStruct,
    #[error("#[WpContextualOption] doesn't have any contexts. Did you forget to add #[WpContext] attribute?")]
    WpContextualOptionWithoutWpContext,
}

impl WpContextualParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
enum WpContextualParseAttrError {
    // It's possible to trigger this error by using something like `#[wp_contextual::WpContext]`,
    // however that's not a valid syntax. There is probably no valid syntax that uses `::` in the
    // current setup, but in case we are missing anything, we should be able to improve the
    // messaging by asking it to be reported.
    #[error("Expected #[WpContext] or #[WpContextualField], found multi-segment path.\nPlease report this case to the `wp_contextual` developers.")]
    UnexpectedAttrPathSegmentCount,
    #[error("Did you mean ','?")]
    UnexpectedPunct,
    #[error("Expected 'edit', 'embed' or 'view', found '{}'", input)]
    UnexpectedWpContextIdent { input: String },
    // syn::Meta::Path or syn::Meta::NameValue
    #[error("Expected #[WpContext(edit, embed, view)]. Did you forget to add context types?")]
    MissingWpContextMeta,
    #[error("Expected #[WpContext(edit, embed, view)], found unsupported tokens")]
    UnexpectedToken,
    #[error("Expected #[WpContext(edit, embed, view)]. Try removing the quotation?")]
    UnexpectedLiteralToken,
}

impl WpContextualParseAttrError {
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
            WpContextualParseError::WpContextualFieldTypeNotSupported.to_string()
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
