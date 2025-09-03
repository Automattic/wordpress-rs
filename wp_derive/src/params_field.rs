use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Attribute, DeriveInput, Lit, Meta, parse_macro_input, spanned::Spanned};

const ATTR_FIELD_NAME: &str = "field_name";
const ATTR_PAGINATION: &str = "supports_pagination";
const ATTR_FROM_QUERY_METHOD: &str = "from_query_method";
const ATTR_APPEND_QUERY_CUSTOM: &str = "append_query_custom";

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
    pub from_query_method_override: Option<String>,
    pub append_query_custom_override: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum FromQueryMethod {
    Get,
    GetCsv,
    GetWpDateTime,
}

#[derive(Debug, Clone, Copy)]
enum AppendQueryMethod {
    AppendValue,
    AppendOption,
    AppendVec,
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

                        // Parse field attributes
                        let field_name_override = Self::parse_field_name_attribute(&field.attrs)?;
                        let from_query_method_override =
                            Self::parse_from_query_method_attribute(&field.attrs)?;
                        let append_query_custom_override =
                            Self::parse_append_query_custom_attribute(&field.attrs)?;

                        Ok(ParsedField {
                            field_ident,
                            field_type: field.ty.clone(),
                            field_name_override,
                            from_query_method_override,
                            append_query_custom_override,
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

    fn parse_from_query_method_attribute(attrs: &[Attribute]) -> syn::Result<Option<String>> {
        let mut from_query_method_override = None;

        for attr in attrs {
            if attr.path().is_ident(ATTR_FROM_QUERY_METHOD) {
                if from_query_method_override.is_some() {
                    return Err(WpDeriveParamsFieldError::DuplicateFromQueryMethodAttribute
                        .into_syn_error(attr.span()));
                }

                let literal: syn::LitStr = attr.parse_args()?;
                from_query_method_override = Some(literal.value());
            }
        }

        Ok(from_query_method_override)
    }

    fn parse_append_query_custom_attribute(attrs: &[Attribute]) -> syn::Result<Option<String>> {
        let mut append_query_custom_override = None;

        for attr in attrs {
            if attr.path().is_ident(ATTR_APPEND_QUERY_CUSTOM) {
                if append_query_custom_override.is_some() {
                    return Err(
                        WpDeriveParamsFieldError::DuplicateAppendQueryCustomAttribute
                            .into_syn_error(attr.span()),
                    );
                }

                let literal: syn::LitStr = attr.parse_args()?;
                append_query_custom_override = Some(literal.value());
            }
        }

        Ok(append_query_custom_override)
    }

    fn generate_all(&self) -> TokenStream {
        let enum_tokens = self.generate_params_field_enum();
        let append_trait_tokens = self.generate_append_url_query_pairs_impl();
        let from_trait_tokens = self.generate_from_url_query_pairs_impl();

        quote! {
            #enum_tokens
            #append_trait_tokens
            #from_trait_tokens
        }
    }

    /// Generate the params field enum from the parsed struct
    fn generate_params_field_enum(&self) -> TokenStream {
        let enum_name = self.generate_enum_name();
        let enum_variants = self.generate_enum_variants();

        quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, strum_macros::IntoStaticStr)]
            pub enum #enum_name {
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

    /// Generate AppendUrlQueryPairs trait implementation
    fn generate_append_url_query_pairs_impl(&self) -> TokenStream {
        let struct_name = &self.struct_ident;

        // Generate method calls for each field
        let method_calls: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| self.generate_append_method_call(field))
            .collect();

        quote! {
            impl AppendUrlQueryPairs for #struct_name {
                fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
                    query_pairs_mut
                        #(.#method_calls)*;
                }
            }
        }
    }

    /// Generate the appropriate append method call for a field
    fn generate_append_method_call(&self, field: &ParsedField) -> TokenStream {
        let field_ident = &field.field_ident;
        let enum_name = self.generate_enum_name();
        let variant_name = self.generate_variant_name(&field.field_ident);

        // If there's a custom override, use it directly
        if let Some(custom_expr) = &field.append_query_custom_override {
            let custom_tokens: TokenStream = custom_expr.parse().unwrap_or_else(|_| {
                quote! { compile_error!("Invalid append_query_custom expression") }
            });
            return quote! {
                append_option_query_value_pair(
                    #enum_name::#variant_name,
                    #custom_tokens
                )
            };
        }

        // Otherwise use auto-detection
        match field.detect_append_query_method() {
            AppendQueryMethod::AppendOption => {
                quote! {
                    append_option_query_value_pair(
                        #enum_name::#variant_name,
                        self.#field_ident.as_ref()
                    )
                }
            }
            AppendQueryMethod::AppendVec => {
                quote! {
                    append_vec_query_value_pair(
                        #enum_name::#variant_name,
                        &self.#field_ident
                    )
                }
            }
            AppendQueryMethod::AppendValue => {
                quote! {
                    append_query_value_pair(
                        #enum_name::#variant_name,
                        &self.#field_ident
                    )
                }
            }
        }
    }

    /// Generate FromUrlQueryPairs trait implementation
    fn generate_from_url_query_pairs_impl(&self) -> TokenStream {
        let struct_name = &self.struct_ident;
        let pagination = self.pagination;

        // Generate field assignments for each field
        let field_assignments: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|field| self.generate_from_query_assignment(field))
            .collect();

        quote! {
            impl FromUrlQueryPairs for #struct_name {
                fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
                    Some(Self {
                        #(#field_assignments,)*
                    })
                }

                fn supports_pagination() -> bool {
                    #pagination
                }
            }
        }
    }

    /// Generate the appropriate query method call for a field
    fn generate_from_query_assignment(&self, field: &ParsedField) -> TokenStream {
        let field_ident = &field.field_ident;
        let enum_name = self.generate_enum_name();
        let variant_name = self.generate_variant_name(&field.field_ident);

        // If there's a method override, use it directly
        if let Some(custom_method) = &field.from_query_method_override {
            let method_ident: Ident = format_ident!("{}", custom_method);
            return quote! {
                #field_ident: query_pairs.#method_ident(#enum_name::#variant_name)
            };
        }

        // Otherwise use auto-detection
        match field.detect_from_query_method() {
            FromQueryMethod::Get => {
                quote! {
                    #field_ident: query_pairs.get(#enum_name::#variant_name)
                }
            }
            FromQueryMethod::GetCsv => {
                quote! {
                    #field_ident: query_pairs.get_csv(#enum_name::#variant_name)
                }
            }
            FromQueryMethod::GetWpDateTime => {
                quote! {
                    #field_ident: query_pairs.get_wp_date_time(#enum_name::#variant_name)
                }
            }
        }
    }
}

impl ParsedField {
    /// Detect the appropriate FromUrlQueryPairs method based on field type
    fn detect_from_query_method(&self) -> FromQueryMethod {
        match &self.field_type {
            syn::Type::Path(type_path) => {
                if let Some(last_segment) = type_path.path.segments.last() {
                    match last_segment.ident.to_string().as_str() {
                        "Option" => {
                            // Check if it's Option<WpGmtDateTime>
                            if self.is_wp_date_time_option(last_segment) {
                                FromQueryMethod::GetWpDateTime
                            } else {
                                FromQueryMethod::Get
                            }
                        }
                        "Vec" => FromQueryMethod::GetCsv,
                        _ => FromQueryMethod::Get, // fallback for non-generic types
                    }
                } else {
                    FromQueryMethod::Get
                }
            }
            _ => FromQueryMethod::Get, // fallback for other types
        }
    }

    /// Detect the appropriate AppendUrlQueryPairs method based on field type
    fn detect_append_query_method(&self) -> AppendQueryMethod {
        match &self.field_type {
            syn::Type::Path(type_path) => {
                if let Some(last_segment) = type_path.path.segments.last() {
                    match last_segment.ident.to_string().as_str() {
                        "Option" => AppendQueryMethod::AppendOption,
                        "Vec" => AppendQueryMethod::AppendVec,
                        _ => AppendQueryMethod::AppendValue, // fallback for non-generic types
                    }
                } else {
                    AppendQueryMethod::AppendValue
                }
            }
            _ => AppendQueryMethod::AppendValue, // fallback for other types
        }
    }

    /// Check if this field is Option<WpGmtDateTime>
    fn is_wp_date_time_option(&self, option_segment: &syn::PathSegment) -> bool {
        // Check if Option has generic arguments
        if let syn::PathArguments::AngleBracketed(ref args) = option_segment.arguments {
            if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                if let syn::Type::Path(inner_path) = inner_type {
                    if let Some(last_segment) = inner_path.path.segments.last() {
                        return last_segment.ident == "WpGmtDateTime";
                    }
                }
            }
        }
        false
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
    #[error("Duplicate #[from_query_method] attribute found")]
    DuplicateFromQueryMethodAttribute,
    #[error("Duplicate #[append_query_custom] attribute found")]
    DuplicateAppendQueryCustomAttribute,
    #[error("Duplicate #[supports_pagination] attribute found")]
    DuplicatePaginationAttribute,
    #[error("#[supports_pagination] attribute is required")]
    PaginationAttributeRequired,
    #[error("#[supports_pagination] attribute requires a boolean value")]
    PaginationRequiresValue,
    #[error("#[supports_pagination] attribute must be a boolean (true or false)")]
    PaginationMustBeBool,
}

impl WpDeriveParamsFieldError {
    fn into_syn_error(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new(span, self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_type_detection_option_u32() {
        let field_type: syn::Type = syn::parse2(quote! { Option<u32> }).unwrap();
        let field = ParsedField {
            field_ident: format_ident!("page"),
            field_type,
            field_name_override: None,
            from_query_method_override: None,
            append_query_custom_override: None,
        };

        assert!(matches!(
            field.detect_from_query_method(),
            FromQueryMethod::Get
        ));
        assert!(matches!(
            field.detect_append_query_method(),
            AppendQueryMethod::AppendOption
        ));
    }

    #[test]
    fn test_type_detection_vec_i64() {
        let field_type: syn::Type = syn::parse2(quote! { Vec<i64> }).unwrap();
        let field = ParsedField {
            field_ident: format_ident!("author"),
            field_type,
            field_name_override: None,
            from_query_method_override: None,
            append_query_custom_override: None,
        };

        assert!(matches!(
            field.detect_from_query_method(),
            FromQueryMethod::GetCsv
        ));
        assert!(matches!(
            field.detect_append_query_method(),
            AppendQueryMethod::AppendVec
        ));
    }

    #[test]
    fn test_type_detection_option_wp_gmt_date_time() {
        let field_type: syn::Type = syn::parse2(quote! { Option<WpGmtDateTime> }).unwrap();
        let field = ParsedField {
            field_ident: format_ident!("after"),
            field_type,
            field_name_override: None,
            from_query_method_override: None,
            append_query_custom_override: None,
        };

        assert!(matches!(
            field.detect_from_query_method(),
            FromQueryMethod::GetWpDateTime
        ));
        assert!(matches!(
            field.detect_append_query_method(),
            AppendQueryMethod::AppendOption
        ));
    }

    #[test]
    fn test_type_detection_plain_type() {
        let field_type: syn::Type = syn::parse2(quote! { String }).unwrap();
        let field = ParsedField {
            field_ident: format_ident!("name"),
            field_type,
            field_name_override: None,
            from_query_method_override: None,
            append_query_custom_override: None,
        };

        assert!(matches!(
            field.detect_from_query_method(),
            FromQueryMethod::Get
        ));
        assert!(matches!(
            field.detect_append_query_method(),
            AppendQueryMethod::AppendValue
        ));
    }
}
