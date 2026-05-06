use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseBlockPattern {
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub content: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub viewport_width: Option<i64>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub inserter: Option<bool>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub categories: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub keywords: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub block_types: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub post_types: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub template_types: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub source: Option<String>,
}
