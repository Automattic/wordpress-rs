use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced,
    parse::{Parse, ParseBuffer, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Attribute, Field, Token,
};

pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_struct = parse_macro_input!(input as ParsedStruct);
    TokenStream::from_iter([
        parsed_struct.generate_cloned_type_with_new_name(),
        parsed_struct.generate_all_fields_set_to_none_implementation(),
        parsed_struct.generate_from_implementation_for_cloned_type(),
        parsed_struct.generate_custom_deserializer(),
    ])
    .into()
}

#[derive(Debug)]
struct ParsedStruct {
    attrs: Vec<Attribute>,
    struct_ident: Ident,
    fields: Punctuated<Field, Token![,]>,
}

impl ParsedStruct {
    fn cloned_type_ident(&self) -> Ident {
        format_ident!("DeserializeHelper{}", self.struct_ident.to_string())
    }

    fn generate_cloned_type_with_new_name(&self) -> TokenStream {
        let attrs = &self.attrs;
        let fields = &self.fields;
        let ident = format_ident!("{}", self.cloned_type_ident());
        quote! {
            #[derive(Debug, serde::Deserialize)]
            #(#attrs)*
            struct #ident {
                #fields
            }
        }
    }

    fn generate_all_fields_set_to_none_implementation(&self) -> TokenStream {
        let cloned_type_ident = &self.cloned_type_ident();
        let f = self.fields.iter().map(|f| {
            if let Some(field_ident) = &f.ident {
                quote! {
                    #field_ident: None
                }
            } else {
                panic!("All fields should have an ident");
            }
        });
        quote! {
            impl #cloned_type_ident {
                fn all_fields_none() -> Self {
                    Self {
                        #(#f,)*
                    }
                }
            }
        }
    }

    fn generate_from_implementation_for_cloned_type(&self) -> TokenStream {
        let struct_ident = &self.struct_ident;
        let cloned_type_ident = &self.cloned_type_ident();
        let f = self.fields.iter().map(|f| {
            if let Some(field_ident) = &f.ident {
                quote! {
                    #field_ident: value.#field_ident
                }
            } else {
                panic!("All fields should have an ident");
            }
        });
        quote! {
            impl From<#cloned_type_ident> for #struct_ident {
                fn from(value: #cloned_type_ident) -> Self {
                    Self {
                        #(#f,)*
                    }
                }
            }
        }
    }

    fn generate_custom_deserializer(&self) -> TokenStream {
        let struct_ident = &self.struct_ident;
        let cloned_type_ident = &self.cloned_type_ident();
        quote! {
            impl<'de> serde::Deserialize<'de> for #struct_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    deserializer
                        .deserialize_any(wp_serde_helper::DeserializeEmptyVecOrT::<#cloned_type_ident>::new(
                            Box::new(|| {
                                #cloned_type_ident::all_fields_none()
                            })
                        ))
                        .map(|internal| Self::from(internal))
                }
            }
        }
    }
}

impl ParsedStruct {
    fn parse_fields(content: ParseBuffer) -> syn::Result<Punctuated<Field, Comma>> {
        let fields = content.parse_terminated(Field::parse_named, Token![,])?;

        let non_optional_field_type = fields.iter().find_map(|f| {
            match &f.ty {
                syn::Type::Path(type_path) => {
                    let first_segment = &type_path.path.segments[0];

                    // `Option` type has only one segment with an ident `Option`
                    if type_path.path.segments.len() != 1 || first_segment.ident != "Option" {
                        Some(&f.ty)
                    } else {
                        None
                    }
                }
                _ => Some(&f.ty),
            }
        });

        if let Some(non_optional_field_type) = non_optional_field_type {
            let mut original_type = non_optional_field_type.to_token_stream().to_string();
            original_type.retain(|c| !c.is_whitespace());
            return Err(WpDeserializeParseError::NonOptionalField { original_type }
                .into_syn_error(non_optional_field_type.span()));
        }

        Ok(fields)
    }

    // Fails if attributes include `#[serde(transparent)]`
    fn parse_outer(input: ParseStream) -> syn::Result<Vec<Attribute>> {
        let attrs = Attribute::parse_outer(input)?;
        for attr in &attrs {
            if let syn::Meta::List(meta_list) = &attr.meta {
                if let Some(ident) = meta_list.path.get_ident() {
                    if *ident != "serde" {
                        continue;
                    }

                    if let Some(proc_macro2::TokenTree::Ident(first_token_ident)) =
                        meta_list.tokens.clone().into_iter().next()
                    {
                        if first_token_ident.to_string().as_str() == "transparent" {
                            return Err(
                                WpDeserializeParseError::SerdeTransparentAttributeNotSupported
                                    .into_syn_error(first_token_ident.span()),
                            );
                        }
                    }
                }
            }
        }
        Ok(attrs)
    }
}

impl Parse for ParsedStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = Self::parse_outer(input)?;

        let _vis: syn::Visibility = input.parse()?;
        let _struct_token: Token![struct] = input.parse()?;
        let struct_ident: Ident = input.parse()?;
        let content: ParseBuffer;
        let _brace_token = braced!(content in input);
        let fields = Self::parse_fields(content)?;

        Ok(Self {
            attrs,
            struct_ident,
            fields,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum WpDeserializeParseError {
    #[error(
        "`WpDeserialize` only supports optional fields, consired replacing with `Option<{}>`",
        original_type
    )]
    NonOptionalField { original_type: String },
    #[error("{}", SERDE_TRANSPARENT_PARSING_ERROR)]
    SerdeTransparentAttributeNotSupported,
}

impl WpDeserializeParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}

const SERDE_TRANSPARENT_PARSING_ERROR: &str = r#"`#[serde(transparent)]` attribute is not supported.

`wp_derive::WpDeserialize` and `#[serde(transparent)]` can't be combined. However, you can use `wp_serde_helper::DeserializeEmptyVecOrT` to manually replicate the same behaviour.

Here is an example of how to do this:
https://github.com/Automattic/wordpress-rs/pull/532/commits/1027e6ed6c8bd0e4cd9aec5ec0595f64f5e925b7#diff-139286995fa3539fe509c87c0b832e974c87c687d64330c6a9c0bd117cdf54f7"#;
