use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseBlockPatternCategory {
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub label: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub description: Option<String>,
}
