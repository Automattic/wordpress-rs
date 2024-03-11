use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, DeriveInput, Ident};

const CONTEXTS: [&str; 3] = ["Edit", "Embed", "View"];
const IDENT_PREFIX: &str = "Sparse";

#[proc_macro_derive(WPContextual, attributes(WPContext))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);
    let original_ident = &ast.ident;
    let ident_name_without_prefix = match ident_name_without_prefix(original_ident) {
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

    let mut token_stream = proc_macro2::TokenStream::new();
    CONTEXTS.iter().for_each(|context| {
        let cname = ident_name_for_context(&ident_name_without_prefix, context);
        let cident = Ident::new(&cname, original_ident.span());
        let non_optional_fields =
            filtered_fields_for_context(fields.iter(), format!("\"{}\"", context.to_lowercase()))
                .map(|f| {
                    let new_type = extract_inner_type_of_option(&f.ty).unwrap_or(f.ty.clone());
                    syn::Field {
                        // Remove the WPContext attribute from the generated field
                        attrs: attrs_without_wp_context(f.attrs.clone()),
                        vis: f.vis.clone(),
                        mutability: syn::FieldMutability::None,
                        ident: f.ident.clone(),
                        colon_token: f.colon_token,
                        ty: new_type,
                    }
                });
        token_stream.extend(quote! {
            #[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record)]
            pub struct #cident {
                #(#non_optional_fields,)*
            }
        });
    });
    token_stream.into()
}

fn filtered_fields_for_context<'a>(
    fields: impl Iterator<Item = &'a syn::Field>,
    context: String,
) -> impl Iterator<Item = &'a syn::Field> {
    fields.filter(move |f| {
        for attr in &f.attrs {
            if attr.path().segments.len() == 1
                && is_wp_context_ident(&attr.path().segments[0].ident)
            {
                if let syn::Meta::List(meta_list) = &attr.meta {
                    return meta_list.tokens.clone().into_iter().any(|t| {
                        if let proc_macro2::TokenTree::Literal(l) = t {
                            l.to_string() == context
                        } else {
                            false
                        }
                    });
                }
            }
        }
        false
    })
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

fn ident_name_without_prefix(ident: &Ident) -> Result<String, syn::Error> {
    let ident_name = ident.to_string();
    let incorrect_ident_error = syn::Error::new(ident.span(), incorrect_ident_name_error_message());
    let ident_name_without_prefix = ident_name
        .strip_prefix(IDENT_PREFIX)
        .ok_or_else(|| incorrect_ident_error.clone())?;
    syn::parse_str::<Ident>(ident_name_without_prefix)
        .map_err(|_| incorrect_ident_error.clone())?;
    Ok(ident_name_without_prefix.to_string())
}

fn ident_name_for_context(ident_name_without_prefix: &String, context: &str) -> String {
    format!("{}With{}Context", ident_name_without_prefix, context)
}

fn is_wp_context_ident(ident: &Ident) -> bool {
    ident.to_string().eq("WPContext")
}

fn attrs_without_wp_context(attrs: Vec<Attribute>) -> Vec<Attribute> {
    attrs
        .into_iter()
        .filter(|attr| {
            !attr
                .meta
                .path()
                .segments
                .iter()
                .any(|s| is_wp_context_ident(&s.ident))
        })
        .collect()
}

fn incorrect_ident_name_error_message() -> String {
    format!("Original Struct names need to start with '{}' prefix. This prefix will be removed from the generated Structs, so it needs to be followed up with a proper Rust type name, starting with an uppercase letter.", IDENT_PREFIX)
}
