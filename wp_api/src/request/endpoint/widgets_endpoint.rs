use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    SparseField,
    widgets::{
        SparseWidgetFieldWithEditContext, SparseWidgetFieldWithEmbedContext,
        SparseWidgetFieldWithViewContext, WidgetId, WidgetListParams, WidgetWithEditContext,
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum WidgetsRequest {
    #[contextual_paged(url = "/widgets", params = &WidgetListParams, output = Vec<crate::widgets::SparseWidget>, filter_by = crate::widgets::SparseWidgetField)]
    List,
    #[contextual_get(url = "/widgets/<widget_id>", output = crate::widgets::SparseWidget, filter_by = crate::widgets::SparseWidgetField)]
    Retrieve,
    #[post(url = "/widgets", params = &crate::widgets::WidgetCreateParams, output = WidgetWithEditContext)]
    Create,
    #[post(url = "/widgets/<widget_id>", params = &crate::widgets::WidgetUpdateParams, output = WidgetWithEditContext)]
    Update,
    #[delete(url = "/widgets/<widget_id>", output = crate::widgets::WidgetDeleteResponse)]
    Delete,
}

impl DerivedRequest for WidgetsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match &self {
            Self::Delete => vec![("force", true.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(
    SparseWidgetFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseWidgetFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseWidgetFieldWithViewContext
);
