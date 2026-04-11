use crate::impl_as_query_value_for_new_type;
use serde::{Deserialize, Serialize};

impl_as_query_value_for_new_type!(SegmentId);
uniffi::custom_newtype!(SegmentId, u64);
/// Identifier of a WordPress.com content segment (e.g. "Blog", "Business").
///
/// The set of valid IDs is hand-curated on the server and is not a dense
/// range — fetch `/wpcom/v2/segments` to discover the currently valid IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentId(pub u64);

/// A WordPress.com content segment returned from `/wpcom/v2/segments`.
///
/// Segments are curated verticals (e.g. "Blog", "Business") used by the
/// site-creation and domain-suggestion flows to tailor recommendations.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct Segment {
    /// Stable numeric identifier for the segment.
    pub id: SegmentId,
    /// Stable machine-readable slug (e.g. `"blog"`, `"business"`).
    /// More robust for client-side feature detection than the numeric `id`.
    pub slug: String,
    /// Whether this segment is shown in mobile clients.
    pub mobile: bool,
    /// Human-readable segment title (e.g. `"Blog"`).
    #[serde(rename = "segment_type_title")]
    pub title: String,
    /// Human-readable segment subtitle/description.
    #[serde(rename = "segment_type_subtitle")]
    pub subtitle: String,
    /// Public CDN URL of the segment icon.
    #[serde(rename = "icon_URL")]
    pub icon_url: String,
    /// Hex color string associated with the icon (e.g. `"#3d4145"`).
    pub icon_color: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segments_deserialization_all() {
        let file =
            std::fs::File::open("tests/wpcom/segments/all.json").expect("Failed to open file");
        let segments: Vec<Segment> = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(segments.len(), 5);

        let blog = segments
            .iter()
            .find(|s| s.slug == "blog")
            .expect("blog segment missing");
        assert_eq!(blog.id, SegmentId(2));
        assert!(blog.mobile);
        assert_eq!(blog.title, "Blog");
        assert_eq!(
            blog.subtitle,
            "Share and discuss ideas, updates, or creations."
        );
        assert_eq!(
            blog.icon_url,
            "https://s.wp.com/i/mobile_segmentation_icons/monochrome/ic_blogger.png"
        );
        assert_eq!(blog.icon_color, "#3d4145");

        let online_store = segments
            .iter()
            .find(|s| s.slug == "online-store")
            .expect("online-store segment missing");
        assert_eq!(online_store.id, SegmentId(3));
        assert!(!online_store.mobile);
    }
}
