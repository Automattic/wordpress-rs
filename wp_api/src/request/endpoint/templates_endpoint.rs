use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::templates::{
    SparseTemplateFieldWithEditContext, SparseTemplateFieldWithEmbedContext,
    SparseTemplateFieldWithViewContext,
};
use crate::SparseField;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TemplatesRequest {
    #[contextual_get(url = "/templates", params = &crate::templates::TemplateListParams, output = Vec<crate::templates::SparseTemplate>, filter_by = crate::templates::SparseTemplateField)]
    List,
}

impl DerivedRequest for TemplatesRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

impl SparseField for SparseTemplateFieldWithEditContext {
    fn as_str(&self) -> &str {
        match self {
            Self::TemplateType => "type",
            Self::PostId => "wp_id",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseTemplateFieldWithEmbedContext {
    fn as_str(&self) -> &str {
        match self {
            Self::TemplateType => "type",
            Self::PostId => "wp_id",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseTemplateFieldWithViewContext {
    fn as_str(&self) -> &str {
        match self {
            Self::TemplateType => "type",
            Self::PostId => "wp_id",
            _ => self.as_field_name(),
        }
    }
}
