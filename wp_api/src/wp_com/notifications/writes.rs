//! Marking notes read, unread and seen.
//!
//! A note's read state is a count of unread updates the server keeps. Marking a note read lowers
//! it by a positive amount, down to none; marking it unread raises it with a negative amount.
//! Every parameter of these requests goes in the body.

use super::{NotificationId, lenient};
use crate::date::WpGmtDateTime;
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};

/// The amount that marks a note read: more than any note's count of unread updates. The WordPress
/// apps for iOS and Android send it.
pub const NOTIFICATION_READ_COUNT: i64 = 9999;

/// The amount that marks a note unread, as WordPress.com's web notifications panel sends it.
pub const NOTIFICATION_UNREAD_COUNT: i64 = -1;

/// A note to mark read or unread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct NotificationReadChange {
    pub note_id: NotificationId,
    /// `true` to mark the note read, `false` to mark it unread.
    pub read: bool,
}

/// The body of `POST /notifications/read`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationsReadParams {
    /// The notes to mark. They can mix read and unread. Sent as `counts`, each note's ID with
    /// [`NOTIFICATION_READ_COUNT`] or [`NOTIFICATION_UNREAD_COUNT`].
    pub changes: Vec<NotificationReadChange>,
    /// Which kind of client is asking: `mobile`, `notifications-panel` or
    /// `reader-conversations`. The API uses it for its stats only.
    #[uniffi(default = None)]
    pub source: Option<String>,
}

impl Serialize for NotificationsReadParams {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        struct Counts<'a>(&'a [NotificationReadChange]);
        impl Serialize for Counts<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut map = serializer.serialize_map(Some(self.0.len()))?;
                for change in self.0 {
                    let count = if change.read {
                        NOTIFICATION_READ_COUNT
                    } else {
                        NOTIFICATION_UNREAD_COUNT
                    };
                    map.serialize_entry(&change.note_id.0.to_string(), &count)?;
                }
                map.end()
            }
        }

        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("counts", &Counts(&self.changes))?;
        if let Some(source) = &self.source {
            map.serialize_entry("source", source)?;
        }
        map.end()
    }
}

/// The answer to `POST /notifications/read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationsReadResponse {
    /// Whether the server made the change. `false` when the answer doesn't say.
    #[serde(default, deserialize_with = "flag_or_false")]
    pub success: bool,
    /// The notes of the request that are unread afterwards. `None` when the answer has no list.
    #[serde(default, deserialize_with = "ids")]
    #[uniffi(default = None)]
    pub updated: Option<Vec<NotificationId>>,
}

/// The body of `POST /notifications/read/all`.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct NotificationsReadAllParams {
    /// Only notes at or before this second are marked read, so a note that arrived after the
    /// user asked stays unread. `None` marks every note read. Sent as seconds since 1970.
    #[uniffi(default = None)]
    pub time: Option<WpGmtDateTime>,
    /// As for [`NotificationsReadParams::source`].
    #[uniffi(default = None)]
    pub source: Option<String>,
}

impl Serialize for NotificationsReadAllParams {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_time_and_source(serializer, self.time, self.source.as_deref())
    }
}

/// The answer to `POST /notifications/read/all`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationsReadAllResponse {
    /// Whether the server made the change. `false` when the answer doesn't say.
    #[serde(default, deserialize_with = "flag_or_false")]
    pub success: bool,
    /// How many notes were marked read.
    #[serde(default, deserialize_with = "lenient::unsigned")]
    #[uniffi(default = None)]
    pub count: Option<u64>,
    /// When the user last looked at their notifications.
    #[serde(default, deserialize_with = "lenient::date")]
    #[uniffi(default = None)]
    pub last_seen_time: Option<WpGmtDateTime>,
}

/// The body of `POST /notifications/seen`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationsSeenParams {
    /// The time of the newest note the user has seen. Sent as seconds since 1970: the API reads
    /// any other form as a number, and answers success without moving the time.
    pub time: WpGmtDateTime,
    /// As for [`NotificationsReadParams::source`].
    #[uniffi(default = None)]
    pub source: Option<String>,
}

impl Serialize for NotificationsSeenParams {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_time_and_source(serializer, Some(self.time), self.source.as_deref())
    }
}

/// The answer to `POST /notifications/seen`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationsSeenResponse {
    /// Whether the server took the time. `false` when the answer doesn't say.
    #[serde(default, deserialize_with = "flag_or_false")]
    pub success: bool,
    /// When the user last looked at their notifications, as the server now has it.
    #[serde(default, deserialize_with = "lenient::date")]
    #[uniffi(default = None)]
    pub last_seen_time: Option<WpGmtDateTime>,
}

/// The answer to `GET /me?fields=has_unseen_notes`: the flag that lights the notifications bell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct WpComUnseenNotes {
    /// Whether the account has a note newer than the last time the user looked at their
    /// notifications.
    pub has_unseen_notes: bool,
}

fn serialize_time_and_source<S: Serializer>(
    serializer: S,
    time: Option<WpGmtDateTime>,
    source: Option<&str>,
) -> Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(None)?;
    if let Some(time) = time {
        map.serialize_entry("time", &time.0.timestamp())?;
    }
    if let Some(source) = source {
        map.serialize_entry("source", source)?;
    }
    map.end()
}

fn flag_or_false<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(lenient::flag(d)?.unwrap_or(false))
}

/// A list of IDs, each a number or a string. An entry that isn't an ID is left out. Anything but
/// a list reads as `None`.
fn ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Vec<NotificationId>>, D::Error> {
    let serde_json::Value::Array(values) = serde_json::Value::deserialize(d)? else {
        return Ok(None);
    };
    Ok(Some(
        values
            .iter()
            .filter_map(|value| match value {
                serde_json::Value::Number(number) => number.as_u64(),
                serde_json::Value::String(text) => text.trim().parse().ok(),
                _ => None,
            })
            .filter(|id| *id != 0)
            .map(NotificationId)
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    //! Every ID and time here is invented.

    use super::*;
    use serde_json::json;

    fn date(seconds: i64) -> WpGmtDateTime {
        WpGmtDateTime(chrono::DateTime::from_timestamp(seconds, 0).unwrap())
    }

    #[test]
    fn test_the_body_that_marks_notes_read_and_unread() {
        let params = NotificationsReadParams {
            changes: vec![
                NotificationReadChange {
                    note_id: NotificationId(10_000_000_001),
                    read: true,
                },
                NotificationReadChange {
                    note_id: NotificationId(10_000_000_002),
                    read: false,
                },
            ],
            source: Some("mobile".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            json!({
                "counts": { "10000000001": 9999, "10000000002": -1 },
                "source": "mobile",
            })
        );
        let no_source = NotificationsReadParams {
            source: None,
            ..params
        };
        assert!(
            serde_json::to_value(&no_source)
                .unwrap()
                .get("source")
                .is_none()
        );
    }

    #[test]
    fn test_the_bodies_with_a_time_send_seconds() {
        let read_all = NotificationsReadAllParams {
            time: Some(date(1_800_000_000)),
            source: Some("mobile".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&read_all).unwrap(),
            json!({ "time": 1_800_000_000, "source": "mobile" })
        );
        assert_eq!(
            serde_json::to_value(NotificationsReadAllParams::default()).unwrap(),
            json!({})
        );
        let seen = NotificationsSeenParams {
            time: date(1_800_000_100),
            source: None,
        };
        assert_eq!(
            serde_json::to_value(&seen).unwrap(),
            json!({ "time": 1_800_000_100 })
        );
    }

    #[test]
    fn test_the_answer_to_marking_notes() {
        let answer: NotificationsReadResponse = serde_json::from_value(
            json!({ "success": true, "updated": [10_000_000_002u64, "10000000003", "x", 0] }),
        )
        .unwrap();
        assert!(answer.success);
        assert_eq!(
            answer.updated,
            Some(vec![
                NotificationId(10_000_000_002),
                NotificationId(10_000_000_003)
            ])
        );

        let answer: NotificationsReadResponse =
            serde_json::from_value(json!({ "success": 1 })).unwrap();
        assert!(answer.success);
        assert_eq!(answer.updated, None);

        // An answer that doesn't say it worked didn't.
        let answer: NotificationsReadResponse =
            serde_json::from_value(json!({ "updated": false })).unwrap();
        assert!(!answer.success);
        assert_eq!(answer.updated, None);
    }

    #[test]
    fn test_the_answer_to_marking_every_note() {
        let answer: NotificationsReadAllResponse = serde_json::from_value(
            json!({ "success": true, "count": 3, "last_seen_time": "1800000000" }),
        )
        .unwrap();
        assert_eq!(
            answer,
            NotificationsReadAllResponse {
                success: true,
                count: Some(3),
                last_seen_time: Some(date(1_800_000_000)),
            }
        );
    }

    #[test]
    fn test_the_answer_to_marking_seen() {
        for last_seen_time in [json!(1_800_000_100), json!("1800000100")] {
            let answer: NotificationsSeenResponse = serde_json::from_value(
                json!({ "success": true, "last_seen_time": last_seen_time }),
            )
            .unwrap();
            assert!(answer.success);
            assert_eq!(answer.last_seen_time, Some(date(1_800_000_100)));
        }
        let answer: NotificationsSeenResponse =
            serde_json::from_value(json!({ "success": false })).unwrap();
        assert!(!answer.success);
        assert_eq!(answer.last_seen_time, None);
    }

    #[test]
    fn test_the_unseen_flag() {
        let flag: WpComUnseenNotes =
            serde_json::from_value(json!({ "has_unseen_notes": true })).unwrap();
        assert!(flag.has_unseen_notes);
        // A missing flag is an error, not a clear bell.
        assert!(serde_json::from_value::<WpComUnseenNotes>(json!({})).is_err());
    }
}
