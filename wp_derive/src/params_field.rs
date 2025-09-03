use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Attribute, Field, Token, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

const ATTR_FIELD_NAME: &str = "field_name";

pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_struct = parse_macro_input!(input as ParsedParamsStruct);

    parsed_struct.generate_params_field_enum().into()
}

#[derive(Debug, Clone)]
pub struct ParsedParamsStruct {
    pub struct_ident: Ident,
    pub fields: Vec<ParsedField>,
}

#[derive(Debug, Clone)]
pub struct ParsedField {
    pub field_ident: Ident,
    pub _field_type: syn::Type,
    pub field_name_override: Option<String>,
}

impl Parse for ParsedParamsStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _vis: syn::Visibility = input.parse()?;
        let _struct_token: Token![struct] = input.parse()?;
        let struct_ident: Ident = input.parse()?;

        let content;
        let _brace_token = braced!(content in input);

        let fields = Self::parse_fields(&content)?;

        Ok(Self {
            struct_ident,
            fields,
        })
    }
}

impl ParsedParamsStruct {
    fn parse_fields(content: ParseStream) -> syn::Result<Vec<ParsedField>> {
        let fields: Punctuated<Field, Comma> =
            content.parse_terminated(Field::parse_named, Token![,])?;

        fields
            .into_iter()
            .map(|field| {
                let field_span = field.span();
                let field_ident = field.ident.ok_or_else(|| {
                    WpDeriveParamsFieldError::UnnamedField.into_syn_error(field_span)
                })?;

                // Parse field attributes for #[field_name("...")]
                let field_name_override = Self::parse_field_name_attribute(&field.attrs)?;

                Ok(ParsedField {
                    field_ident,
                    _field_type: field.ty,
                    field_name_override,
                })
            })
            .collect()
    }

    fn parse_field_name_attribute(attrs: &[Attribute]) -> syn::Result<Option<String>> {
        let mut field_name_override = None;

        for attr in attrs {
            if attr.path().is_ident(ATTR_FIELD_NAME) {
                if field_name_override.is_some() {
                    return Err(WpDeriveParamsFieldError::DuplicateFieldNameAttribute
                        .into_syn_error(attr.span()));
                }

                let literal: syn::LitStr = attr.parse_args()?;
                field_name_override = Some(literal.value());
            }
        }

        Ok(field_name_override)
    }

    /// Generate the params field enum from the parsed struct
    fn generate_params_field_enum(&self) -> TokenStream {
        let enum_name = self.generate_enum_name();
        let enum_variants = self.generate_enum_variants();

        quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, strum_macros::IntoStaticStr)]
            enum #enum_name {
                #(#enum_variants,)*
            }
        }
    }

    /// Generate the enum name from struct name (e.g., PostListParams -> PostListParamsField)
    fn generate_enum_name(&self) -> Ident {
        let struct_name = self.struct_ident.to_string();
        format_ident!("{}Field", struct_name)
    }

    /// Generate enum variants with strum attributes
    fn generate_enum_variants(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .map(|field| {
                let variant_name = self.generate_variant_name(&field.field_ident);
                let serialize_name = self.generate_serialize_name(field);

                quote! {
                    #[strum(serialize = #serialize_name)]
                    #variant_name
                }
            })
            .collect()
    }

    /// Generate variant name (-> PascalCase)
    fn generate_variant_name(&self, field_ident: &Ident) -> Ident {
        let field_name = field_ident.to_string();
        let pascal_case = field_name.to_case(Case::Pascal);
        format_ident!("{}", pascal_case)
    }

    /// Generate serialize name (use override or original field name)
    fn generate_serialize_name(&self, field: &ParsedField) -> String {
        field
            .field_name_override
            .clone()
            .unwrap_or_else(|| field.field_ident.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
enum WpDeriveParamsFieldError {
    #[error("Only named fields are supported")]
    UnnamedField,
    #[error("Duplicate #[field_name] attribute found")]
    DuplicateFieldNameAttribute,
}

impl WpDeriveParamsFieldError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
