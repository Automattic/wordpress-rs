use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Attribute, DeriveInput, Lit, Meta, parse_macro_input, spanned::Spanned};

const ATTR_FIELD_NAME: &str = "field_name";
const ATTR_PAGINATION: &str = "pagination";

pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let parsed_struct = ParsedParamsStruct::from_derive_input(input);

    match parsed_struct {
        Ok(parsed) => parsed.generate_all().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[derive(Debug, Clone)]
pub struct ParsedParamsStruct {
    pub struct_ident: Ident,
    pub fields: Vec<ParsedField>,
    pub pagination: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedField {
    pub field_ident: Ident,
    pub field_type: syn::Type,
    pub field_name_override: Option<String>,
}

impl ParsedParamsStruct {
    pub fn from_derive_input(input: DeriveInput) -> syn::Result<Self> {
        let struct_ident = input.ident;

        // Parse pagination attribute from derive input attrs
        let pagination = Self::parse_pagination_attribute(&input.attrs)?;

        // Extract struct fields
        let fields = match input.data {
            syn::Data::Struct(data_struct) => Self::parse_fields_from_struct(&data_struct.fields)?,
            _ => {
                return Err(WpDeriveParamsFieldError::OnlyStructsSupported
                    .into_syn_error(struct_ident.span()));
            }
        };

        Ok(Self {
            struct_ident,
            fields,
            pagination,
        })
    }

    fn parse_fields_from_struct(fields: &syn::Fields) -> syn::Result<Vec<ParsedField>> {
        match fields {
            syn::Fields::Named(named_fields) => {
                named_fields
                    .named
                    .iter()
                    .map(|field| {
                        let field_span = field.span();
                        let field_ident = field.ident.clone().ok_or_else(|| {
                            WpDeriveParamsFieldError::UnnamedField.into_syn_error(field_span)
                        })?;

                        // Parse field attributes for #[field_name("...")]
                        let field_name_override = Self::parse_field_name_attribute(&field.attrs)?;

                        Ok(ParsedField {
                            field_ident,
                            field_type: field.ty.clone(),
                            field_name_override,
                        })
                    })
                    .collect()
            }
            _ => Err(WpDeriveParamsFieldError::OnlyNamedFieldsSupported
                .into_syn_error(proc_macro2::Span::call_site())),
        }
    }

    fn parse_pagination_attribute(attrs: &[Attribute]) -> syn::Result<bool> {
        let mut pagination_value = None;

        for attr in attrs {
            if attr.path().is_ident(ATTR_PAGINATION) {
                if pagination_value.is_some() {
                    return Err(WpDeriveParamsFieldError::DuplicatePaginationAttribute
                        .into_syn_error(attr.span()));
                }

                let meta = attr.meta.clone();
                match meta {
                    Meta::List(list) => {
                        if list.tokens.is_empty() {
                            return Err(WpDeriveParamsFieldError::PaginationRequiresValue
                                .into_syn_error(attr.span()));
                        }

                        let literal: Lit = syn::parse2(list.tokens)?;
                        match literal {
                            Lit::Bool(bool_lit) => {
                                pagination_value = Some(bool_lit.value);
                            }
                            _ => {
                                return Err(WpDeriveParamsFieldError::PaginationMustBeBool
                                    .into_syn_error(attr.span()));
                            }
                        }
                    }
                    _ => {
                        return Err(WpDeriveParamsFieldError::PaginationRequiresValue
                            .into_syn_error(attr.span()));
                    }
                }
            }
        }

        pagination_value.ok_or_else(|| {
            WpDeriveParamsFieldError::PaginationAttributeRequired
                .into_syn_error(proc_macro2::Span::call_site())
        })
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

    fn generate_all(&self) -> TokenStream {
        self.generate_params_field_enum()
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
    #[error("Only named fields are supported")]
    OnlyNamedFieldsSupported,
    #[error("Only structs are supported")]
    OnlyStructsSupported,
    #[error("Duplicate #[field_name] attribute found")]
    DuplicateFieldNameAttribute,
    #[error("Duplicate #[pagination] attribute found")]
    DuplicatePaginationAttribute,
    #[error("#[pagination] attribute is required")]
    PaginationAttributeRequired,
    #[error("#[pagination] attribute requires a boolean value")]
    PaginationRequiresValue,
    #[error("#[pagination] attribute must be a boolean (true or false)")]
    PaginationMustBeBool,
}

impl WpDeriveParamsFieldError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}
