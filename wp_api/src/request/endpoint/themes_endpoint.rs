use crate::themes::{
    SparseThemeFieldWithEditContext, SparseThemeFieldWithEmbedContext,
    SparseThemeFieldWithViewContext, ThemeStylesheet,
};
use crate::SparseField;
use wp_derive_request_builder::WpDerivedRequest;

use super::{AsNamespace, DerivedRequest, WpNamespace};
#[derive(WpDerivedRequest)]
enum ThemesRequest {
    #[contextual_get(url = "/themes", params = &crate::themes::ThemeListParams, output = Vec<crate::themes::SparseTheme>, filter_by = crate::themes::SparseThemeField)]
    List,
    #[contextual_get(url = "/themes/<theme_stylesheet>", output = crate::themes::SparseTheme, filter_by = crate::themes::SparseThemeField)]
    Retrieve,
}

impl DerivedRequest for ThemesRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(
    SparseThemeFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseThemeFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseThemeFieldWithViewContext
);
