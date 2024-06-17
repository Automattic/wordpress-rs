#![allow(unused)]
use std::error;

use proc_macro2::{Literal, TokenStream, TokenTree};
use syn::{
    braced,
    parse::{Parse, ParseBuffer, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Ident, Token,
};

use crate::{sparse_field_attr::SparseFieldAttr, variant_attr::ParsedVariantAttribute};

#[derive(Debug)]
pub struct ParsedEnum {
    sparse_field_attr: SparseFieldAttr,
    enum_ident: Ident,
    variants: Punctuated<ParsedVariant, Comma>,
}

impl Parse for ParsedEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let sparse_field_attr = input.parse()?;
        let _enum_token: Token![enum] = input.parse()?;
        let enum_ident: Ident = input.parse()?;
        let content: ParseBuffer;
        let brace_token = braced!(content in input);
        Ok(Self {
            sparse_field_attr,
            enum_ident,
            variants: content.parse_terminated(ParsedVariant::parse, Token![,])?,
        })
    }
}

#[derive(Debug)]
pub struct ParsedVariant {
    attr: ParsedVariantAttribute,
    variant_ident: Ident,
}

impl Parse for ParsedVariant {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attr = input.parse()?;
        let variant_ident: Ident = input.parse()?;
        Ok(Self {
            attr,
            variant_ident,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RequestType {
    ContextualGet,
    Delete,
    Get,
    Post,
}