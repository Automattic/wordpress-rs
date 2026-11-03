use crate::{
    UserId,
    date::WpDateString,
    posts::PostId,
    template_parts::TemplatePartId,
    templates::{SparseTemplateContentWrapper, SparseTemplateTitleWrapper, TemplateStatus},
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

wp_content_i64_id!(TemplatePartAutosaveId);

// The template part autosave response uses the parent template-parts controller's
// schema plus a `parent` field. Fields match `SparseTemplatePart` with `parent` added.
#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseTemplatePartAutosave {
    #[WpContext(edit, embed, view)]
    pub id: Option<TemplatePartId>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub theme: Option<String>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub template_type: Option<String>,
    #[WpContext(edit, embed, view)]
    pub source: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub origin: Option<String>,
    #[WpContext(edit, embed, view)]
    pub content: Option<SparseTemplateContentWrapper>,
    #[WpContext(edit, embed, view)]
    pub title: Option<SparseTemplateTitleWrapper>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub status: Option<TemplateStatus>,
    #[serde(rename = "wp_id")]
    #[WpContext(edit, embed, view)]
    pub post_id: Option<PostId>,
    #[WpContext(edit, embed, view)]
    pub has_theme_file: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "crate::date::deserialize_optional_date_string"
    )]
    pub modified: Option<WpDateString>,
    #[WpContext(edit, embed, view)]
    pub area: Option<String>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<PostId>,
}
