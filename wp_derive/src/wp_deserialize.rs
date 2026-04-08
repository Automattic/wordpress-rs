use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Field, Token, braced,
    parse::{Parse, ParseBuffer, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_struct = parse_macro_input!(input as ParsedStruct);
    if parsed_struct.has_serde_transparent() {
        match parsed_struct.generate_transparent_deserializer() {
            Ok(tokens) => tokens.into(),
            Err(err) => err.to_compile_error().into(),
        }
    } else {
        TokenStream::from_iter([
            parsed_struct.generate_cloned_type_with_new_name(),
            parsed_struct.generate_all_fields_set_to_none_implementation(),
            parsed_struct.generate_from_implementation_for_cloned_type(),
            parsed_struct.generate_custom_deserializer(),
        ])
        .into()
    }
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

    fn has_serde_transparent(&self) -> bool {
        self.attrs.iter().any(|attr| {
            if let syn::Meta::List(meta_list) = &attr.meta
                && let Some(ident) = meta_list.path.get_ident()
                && *ident == "serde"
            {
                meta_list.tokens.clone().into_iter().any(|token| {
                    matches!(token, proc_macro2::TokenTree::Ident(ident) if ident == "transparent")
                })
            } else {
                false
            }
        })
    }

    /// Extracts the inner type `T` from a field typed `Option<T>`.
    fn extract_option_inner_type(field: &Field) -> Option<&syn::Type> {
        if let syn::Type::Path(type_path) = &field.ty
            && let Some(first_segment) = type_path.path.segments.first()
            && first_segment.ident == "Option"
            && let syn::PathArguments::AngleBracketed(args) = &first_segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            return Some(inner_ty);
        }
        None
    }

    /// Checks whether a type's last path segment is `HashMap`.
    /// Handles both `HashMap<K, V>` and `std::collections::HashMap<K, V>`.
    fn is_hashmap(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty {
            type_path
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "HashMap")
        } else {
            false
        }
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

    /// Generates a custom `Deserialize` impl for `#[serde(transparent)]` structs.
    ///
    /// Standard `WpDeserialize` uses `DeserializeEmptyVecOrT<DeserializeHelper>`, but
    /// `#[serde(transparent)]` on the helper causes `Option<T>::deserialize` to call
    /// `deserialize_option` on a `MapAccessDeserializer`, which can't distinguish
    /// null from present. Instead, we generate an inline visitor that resolves the
    /// `Option` directly at the `visit_map`/`visit_seq` dispatch point.
    ///
    /// Only supports `Option<HashMap<K, V>>` fields — the generated visitor only
    /// handles `visit_map` (for map inputs) and `visit_seq` (for empty arrays).
    fn generate_transparent_deserializer(&self) -> syn::Result<TokenStream> {
        let struct_ident = &self.struct_ident;
        let field = self
            .fields
            .first()
            .expect("transparent struct must have exactly one field");
        let field_ident = field
            .ident
            .as_ref()
            .expect("transparent field must have an ident");
        let inner_type =
            Self::extract_option_inner_type(field).expect("transparent field must be Option<T>");

        if !Self::is_hashmap(inner_type) {
            return Err(
                WpDeserializeParseError::TransparentRequiresHashMap.into_syn_error(field.ty.span())
            );
        }

        let visitor_ident = format_ident!("{}TransparentVisitor", self.struct_ident);

        Ok(quote! {
            struct #visitor_ident;

            impl<'de> serde::de::Visitor<'de> for #visitor_ident {
                type Value = #struct_ident;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("empty array or map")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    if serde::de::SeqAccess::next_element::<Self::Value>(&mut seq)?.is_none() {
                        Ok(#struct_ident { #field_ident: None })
                    } else {
                        Err(serde::de::Error::invalid_type(
                            serde::de::Unexpected::Seq,
                            &self,
                        ))
                    }
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>,
                {
                    // Safe to wrap in Some: deserialize_any dispatches visit_map only
                    // for actual map inputs, never for null. The null-vs-present
                    // decision is already made at the deserialize_any call site.
                    serde::Deserialize::deserialize(
                        serde::de::value::MapAccessDeserializer::new(map),
                    )
                    .map(|inner: #inner_type| #struct_ident { #field_ident: Some(inner) })
                }
            }

            impl<'de> serde::Deserialize<'de> for #struct_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    // Uses deserialize_any so the format-level deserializer (e.g. serde_json)
                    // dispatches to visit_map/visit_seq based on the actual input token.
                    // This is what makes the Some-wrapping in visit_map safe — see comment there.
                    deserializer.deserialize_any(#visitor_ident)
                }
            }
        })
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
}

impl Parse for ParsedStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

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
    #[error(
        "`WpDeserialize` with `#[serde(transparent)]` only supports `Option<HashMap<K, V>>` fields. \
         The generated deserializer uses `visit_map` to deserialize the inner type from a map input, \
         which only works for map-deserializable types like `HashMap`."
    )]
    TransparentRequiresHashMap,
}

impl WpDeserializeParseError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
