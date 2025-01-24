use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseBuffer, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
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

impl Parse for ParsedStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        let _vis: syn::Visibility = input.parse()?;
        let _struct_token: Token![struct] = input.parse()?;
        let struct_ident: Ident = input.parse()?;
        let content: ParseBuffer;
        let _brace_token = braced!(content in input);
        let fields = content.parse_terminated(Field::parse_named, Token![,])?;
        Ok(Self {
            attrs,
            struct_ident,
            fields,
        })
    }
}
