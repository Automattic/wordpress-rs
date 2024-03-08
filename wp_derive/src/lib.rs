use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

#[proc_macro_derive(WPContextual, attributes(WPContext))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let cname = format!("{}WithEditContext", name);
    let cident = Ident::new(&cname, name.span());
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        unimplemented!("Only implemented for Structs for now");
    };
    let filtered_fields = fields.iter().filter(|f| {
        for attr in &f.attrs {
            if attr.path().segments.len() == 1 && attr.path().segments[0].ident == "WPContext" {
                if let syn::Meta::List(meta_list) = &attr.meta {
                    return meta_list.tokens.clone().into_iter().any(|t| {
                        if let proc_macro2::TokenTree::Literal(l) = t {
                            l.to_string() == "\"view\""
                        } else {
                            false
                        }
                    });
                }
            }
        }
        false
    });
    let non_optional_fields = filtered_fields.map(|f| {
        let new_type = extract_inner_type_of_option(&f.ty).unwrap_or(f.ty.clone());
        syn::Field {
            attrs: Vec::new(),
            vis: f.vis.clone(),
            mutability: syn::FieldMutability::None,
            ident: f.ident.clone(),
            colon_token: f.colon_token,
            ty: new_type,
        }
    });
    quote! {
        pub struct #cident {
            #(#non_optional_fields,)*
        }
    }
    .into()
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
