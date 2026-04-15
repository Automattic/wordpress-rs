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

        let unicorn = segments
            .iter()
            .find(|s| s.slug == "unicorn-ranch")
            .expect("unicorn-ranch segment missing");
        assert_eq!(unicorn.id, SegmentId(101));
        assert!(unicorn.mobile);
        assert_eq!(unicorn.title, "Unicorn Ranch");
        assert_eq!(
            unicorn.subtitle,
            "Manage your mythical creature farm online."
        );
        assert_eq!(
            unicorn.icon_url,
            "https://example.invalid/icons/ic_unicorn.png"
        );
        assert_eq!(unicorn.icon_color, "#ff00ff");

        let bakery = segments
            .iter()
            .find(|s| s.slug == "cloud-bakery")
            .expect("cloud-bakery segment missing");
        assert_eq!(bakery.id, SegmentId(103));
        assert!(!bakery.mobile);
    }
}
