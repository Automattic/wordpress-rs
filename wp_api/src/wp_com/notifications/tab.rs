//! The tabs of the web notifications panel, and which notes each one shows.

use super::{NotificationType, NotificationsQuery, WpComNotification};
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

/// A tab of the notifications list, as the web notifications panel has them.
///
/// Show its name with `localize_notification_tab(value, locale)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum, WpDeriveLocalizable)]
pub enum NotificationTab {
    /// Every note.
    All,
    /// The notes the user hasn't read.
    Unread,
    /// Comments on the user's posts, and replies to the user's comments.
    Comments,
    /// People who subscribed to the user's sites.
    Subscribers,
    /// Likes of the user's posts and comments.
    Likes,
}

impl NotificationTab {
    /// Every tab, in the order the web panel shows them.
    pub const EVERY_TAB: [Self; 5] = [
        Self::All,
        Self::Unread,
        Self::Comments,
        Self::Subscribers,
        Self::Likes,
    ];

    /// The note types the tab shows. Empty for a tab that shows every type.
    pub fn types(&self) -> Vec<NotificationType> {
        match self {
            Self::All | Self::Unread => Vec::new(),
            Self::Comments => vec![NotificationType::Comment],
            Self::Subscribers => vec![NotificationType::Follow],
            Self::Likes => vec![NotificationType::Like, NotificationType::CommentLike],
        }
    }

    /// Whether the tab shows only the notes the user hasn't read.
    pub fn unread_only(&self) -> bool {
        matches!(self, Self::Unread)
    }

    /// Whether the tab shows `note`. A note that doesn't say whether it was read is unread, as
    /// the web panel treats it.
    pub fn includes(&self, note: &WpComNotification) -> bool {
        match self {
            Self::All => true,
            Self::Unread => note.read != Some(true),
            Self::Comments | Self::Subscribers | Self::Likes => note
                .note_type
                .as_ref()
                .is_some_and(|note_type| self.types().contains(note_type)),
        }
    }

    /// Narrows `query` to the notes the tab shows, as the web panel asks for them.
    pub fn narrow(&self, query: NotificationsQuery) -> NotificationsQuery {
        NotificationsQuery {
            types: self.types(),
            unread: self.unread_only(),
            ..query
        }
    }

    /// A name for the tab that never changes, for storing it.
    pub fn key(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Unread => "unread",
            Self::Comments => "comments",
            Self::Subscribers => "subscribers",
            Self::Likes => "likes",
        }
    }
}

impl WpSupportsLocalization for NotificationTab {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            Self::All => WpMessages::notification_tab_all(),
            Self::Unread => WpMessages::notification_tab_unread(),
            Self::Comments => WpMessages::notification_tab_comments(),
            Self::Subscribers => WpMessages::notification_tab_subscribers(),
            Self::Likes => WpMessages::notification_tab_likes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use wp_localization::{WpLocale, WpLocalizable};

    fn note(note_type: &str, read: Option<u8>) -> WpComNotification {
        let mut json = serde_json::json!({ "id": 10_000_000_001_u64, "type": note_type });
        if let Some(read) = read {
            json["read"] = read.into();
        }
        serde_json::from_value(json).unwrap()
    }

    #[rstest]
    #[case::all_shows_any_type(NotificationTab::All, "example_note", Some(1), true)]
    #[case::unread_shows_unread(NotificationTab::Unread, "like", Some(0), true)]
    #[case::unread_hides_read(NotificationTab::Unread, "like", Some(1), false)]
    #[case::unread_shows_a_note_that_does_not_say(NotificationTab::Unread, "like", None, true)]
    #[case::comments_show_comments(NotificationTab::Comments, "comment", Some(1), true)]
    #[case::comments_hide_mentions(NotificationTab::Comments, "automattcher", Some(1), false)]
    #[case::comments_hide_comment_likes(NotificationTab::Comments, "comment_like", Some(1), false)]
    #[case::subscribers_show_follows(NotificationTab::Subscribers, "follow", Some(1), true)]
    #[case::subscribers_hide_fediverse_follows(
        NotificationTab::Subscribers,
        "activitypub_followed_note",
        Some(1),
        false
    )]
    #[case::likes_show_post_likes(NotificationTab::Likes, "like", Some(1), true)]
    #[case::likes_show_comment_likes(NotificationTab::Likes, "comment_like", Some(1), true)]
    #[case::likes_hide_reblogs(NotificationTab::Likes, "reblog", Some(1), false)]
    fn a_tab_shows_the_notes_the_web_panel_does(
        #[case] tab: NotificationTab,
        #[case] note_type: &str,
        #[case] read: Option<u8>,
        #[case] shown: bool,
    ) {
        assert_eq!(tab.includes(&note(note_type, read)), shown);
    }

    #[test]
    fn a_note_with_no_type_is_only_in_all_and_unread() {
        let note: WpComNotification =
            serde_json::from_value(serde_json::json!({ "id": 10_000_000_001_u64, "read": 0 }))
                .unwrap();
        let shown: Vec<NotificationTab> = NotificationTab::EVERY_TAB
            .into_iter()
            .filter(|tab| tab.includes(&note))
            .collect();
        assert_eq!(shown, [NotificationTab::All, NotificationTab::Unread]);
    }

    #[rstest]
    #[case(NotificationTab::All, "")]
    #[case(NotificationTab::Unread, "unread=1")]
    #[case(NotificationTab::Comments, "type=comment")]
    #[case(NotificationTab::Subscribers, "type=follow")]
    #[case(NotificationTab::Likes, "type=like%2Ccomment_like")]
    fn a_tab_asks_for_its_notes_as_the_web_panel_does(
        #[case] tab: NotificationTab,
        #[case] expected: &str,
    ) {
        use crate::url_query::AppendUrlQueryPairs;
        let mut url = url::Url::parse("https://example.com/").unwrap();
        tab.narrow(NotificationsQuery {
            number: Some(5),
            ..Default::default()
        })
        .append_query_pairs(&mut url.query_pairs_mut());
        let expected = if expected.is_empty() {
            "number=5".to_string()
        } else {
            format!("number=5&{expected}")
        };
        assert_eq!(url.query(), Some(expected.as_str()));
    }

    #[test]
    fn each_tab_has_the_web_panels_name() {
        let names: Vec<String> = NotificationTab::EVERY_TAB
            .iter()
            .map(|tab| tab.localize(Some(WpLocale::from("en-US"))))
            .collect();
        assert_eq!(names, ["All", "Unread", "Comments", "Subscribers", "Likes"]);
    }

    #[test]
    fn each_tab_has_its_own_key() {
        let keys: std::collections::HashSet<&str> = NotificationTab::EVERY_TAB
            .iter()
            .map(NotificationTab::key)
            .collect();
        assert_eq!(keys.len(), NotificationTab::EVERY_TAB.len());
    }
}
