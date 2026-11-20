//! Looks at each response as the server sent it, for values that parse but aren't right, and counts
//! the type each field arrives as.
//!
//! Parsing only says a response fits the library's types. A date with a UTC offset of `-16:00`
//! parses, and is wrong. These checks read the raw JSON, so they see what the types leave out too.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Something in a response that isn't right: a value that parses and is wrong, or a post the
/// library couldn't read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// `odd_offset`, `unreadable_date`, `odd_year` or `negative_number` from these checks;
    /// `unreadable_post`, `unknown_card`, `computed_field_panic`, `no_summary_with_words` or
    /// `no_reading_time_with_words` from the sweep.
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<u64>,
    /// Where it is in the post, such as `date` or `featured_media.width`. `post` for a post that
    /// couldn't be read.
    pub field: String,
    /// The value, as the server sent it. For a post that couldn't be read, the reason.
    pub value: String,
    /// For a date: the UTC offset most of the page's other posts have, when there is one. It's what
    /// the odd one would be expected to be.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usual_offset: Option<String>,
}

/// How many posts were looked at, and how many times each field arrived as each type.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Census {
    pub posts: u64,
    pub fields: BTreeMap<String, BTreeMap<&'static str, u64>>,
}

impl Census {
    fn count(&mut self, field: &str, value: &Value) {
        *self
            .fields
            .entry(field.to_string())
            .or_default()
            .entry(shape(value))
            .or_default() += 1;
    }

    pub fn merge(&mut self, other: Census) {
        self.posts += other.posts;
        for (field, shapes) in other.fields {
            let mine = self.fields.entry(field).or_default();
            for (shape, count) in shapes {
                *mine.entry(shape).or_default() += count;
            }
        }
    }
}

/// The type a value arrived as, with the distinctions that have mattered: a number below zero, a
/// string holding a number, and the empty forms PHP sends for "nothing".
pub fn shape(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(true) => "true",
        Value::Bool(false) => "false",
        Value::Number(number) if number.as_f64().is_some_and(|n| n < 0.0) => "negative number",
        Value::Number(number) if number.is_f64() => "fraction",
        Value::Number(_) => "number",
        Value::String(text) if text.is_empty() => "empty string",
        Value::String(text) if text.parse::<f64>().is_ok() => "numeric string",
        Value::String(_) => "string",
        Value::Array(items) if items.is_empty() => "empty array",
        Value::Array(_) => "array",
        Value::Object(map) if map.is_empty() => "empty object",
        Value::Object(_) => "object",
    }
}

/// The objects inside a post whose fields are counted too.
const NESTED: [&str; 3] = ["author", "discussion", "featured_media"];

/// Fields that count or identify something, so they're never below zero. `author.site_ID` isn't
/// here: a Jetpack site sends `-1` for it, which the library reads as no site.
const NEVER_NEGATIVE: [&str; 11] = [
    "ID",
    "site_ID",
    "feed_ID",
    "feed_item_ID",
    "like_count",
    "word_count",
    "author.ID",
    "discussion.comment_count",
    "featured_media.width",
    "featured_media.height",
    "organization_id",
];

/// A UTC offset some place on Earth uses: `-12:00` to `+14:00`, on the hour, half or three quarters.
fn is_real_offset(offset: &str) -> bool {
    let Some((hours, minutes)) = offset.get(1..).and_then(|rest| rest.split_once(':')) else {
        return false;
    };
    let Ok(hours) = hours.parse::<u32>() else {
        return false;
    };
    let limit = if offset.starts_with('-') { 12 } else { 14 };
    match minutes {
        "00" => hours <= limit,
        "30" | "45" => hours < limit,
        _ => false,
    }
}

/// What's wrong with a date, if anything. `false`, `null` and `""` are how the API says there's
/// no date, so they're not wrong.
fn date_problems(value: &Value, this_year: i32) -> Vec<&'static str> {
    let Value::String(text) = value else {
        return vec![];
    };
    if text.is_empty() {
        return vec![];
    }
    let Some((year, offset)) = date_parts(text) else {
        return vec!["unreadable_date"];
    };
    let mut problems = Vec::new();
    if !is_real_offset(offset) {
        problems.push("odd_offset");
    }
    // WordPress's zero date comes through as year 0, which is a known "no date".
    if year > 0 && (year < 1995 || year > this_year + 1) {
        problems.push("odd_year");
    }
    problems
}

/// The parts of a date as the API writes it: `2026-11-16T16:45:34+91:00` gives the year and the
/// offset. `None` if the text isn't in that form.
fn date_parts(text: &str) -> Option<(i32, &str)> {
    let bytes = text.as_bytes();
    let well_formed = bytes.len() >= 25
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && matches!(bytes[19], b'+' | b'-')
        && text[..4].bytes().all(|b| b.is_ascii_digit());
    if !well_formed {
        return None;
    }
    let offset = &text[19..];
    let (hours, minutes) = offset[1..].split_once(':')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    (digits(hours) && digits(minutes) && minutes.len() == 2)
        .then(|| (text[..4].parse().unwrap_or(0), offset))
}

fn id(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

/// Reads a response's posts. Returns what isn't right in them, and the census of their fields.
/// A response that isn't JSON, or has no posts, gives nothing: parsing reports those.
pub fn examine(body: &[u8], this_year: i32) -> (Vec<Finding>, Census) {
    let mut findings = Vec::new();
    let mut census = Census::default();
    let Ok(Value::Object(response)) = serde_json::from_slice::<Value>(body) else {
        return (findings, census);
    };
    // A stream of cards has its posts in its `post` cards.
    let posts: Vec<&Value> = match (response.get("posts"), response.get("cards")) {
        (Some(Value::Array(posts)), _) => posts.iter().collect(),
        (_, Some(Value::Array(cards))) => cards
            .iter()
            .filter(|card| card.get("type").and_then(Value::as_str) == Some("post"))
            .filter_map(|card| card.get("data"))
            .collect(),
        _ => return (findings, census),
    };

    // The offset most of the page's posts have, to say what an odd one should have been.
    let mut offsets: BTreeMap<&str, usize> = BTreeMap::new();
    for post in &posts {
        if let Some((_, offset)) = post
            .get("date")
            .and_then(Value::as_str)
            .and_then(date_parts)
            && is_real_offset(offset)
        {
            *offsets.entry(offset).or_default() += 1;
        }
    }
    let usual_offset = offsets
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(offset, _)| offset.to_string());

    for post in posts {
        let Value::Object(fields) = post else {
            continue;
        };
        census.posts += 1;
        let site_id = id(fields.get("site_ID"));
        let post_id = id(fields.get("ID"));
        let mut found = |kind: &'static str, field: &str, value: String, dated: bool| {
            findings.push(Finding {
                kind,
                site_id,
                post_id,
                field: field.to_string(),
                value,
                usual_offset: if dated { usual_offset.clone() } else { None },
            });
        };
        for (key, value) in fields {
            census.count(key, value);
            if NEVER_NEGATIVE.contains(&key.as_str()) && shape(value) == "negative number" {
                found("negative_number", key, value.to_string(), false);
            }
            if matches!(key.as_str(), "date" | "modified") {
                for kind in date_problems(value, this_year) {
                    found(
                        kind,
                        key,
                        value.as_str().unwrap_or_default().to_string(),
                        true,
                    );
                }
            }
            if NESTED.contains(&key.as_str())
                && let Value::Object(inner) = value
            {
                for (inner_key, inner_value) in inner {
                    let field = format!("{key}.{inner_key}");
                    census.count(&field, inner_value);
                    if NEVER_NEGATIVE.contains(&field.as_str())
                        && shape(inner_value) == "negative number"
                    {
                        found("negative_number", &field, inner_value.to_string(), false);
                    }
                }
            }
            // Each attachment has its own date, written the same way as the post's.
            if key == "attachments"
                && let Value::Object(attachments) = value
            {
                for attachment in attachments.values() {
                    let Some(date) = attachment.get("date") else {
                        continue;
                    };
                    for kind in date_problems(date, this_year) {
                        let text = date.as_str().unwrap_or_default().to_string();
                        found(kind, "attachments.date", text, true);
                    }
                }
            }
        }
    }
    (findings, census)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn examine_posts(posts: Value) -> (Vec<Finding>, Census) {
        examine(json!({ "posts": posts }).to_string().as_bytes(), 2026)
    }

    fn kinds(findings: &[Finding]) -> Vec<(&'static str, &str, &str)> {
        findings
            .iter()
            .map(|f| (f.kind, f.field.as_str(), f.value.as_str()))
            .collect()
    }

    #[test]
    fn real_offsets_are_the_ones_places_use() {
        for offset in [
            "+00:00", "-05:00", "+05:30", "+05:45", "+12:45", "-09:30", "-12:00", "+14:00",
        ] {
            assert!(is_real_offset(offset), "{offset}");
        }
        for offset in [
            "+91:00", "-216:00", "-16:00", "+01:57", "-19:30", "+14:30", "-12:30", "+15:00",
            "+24:00",
        ] {
            assert!(!is_real_offset(offset), "{offset}");
        }
    }

    #[test]
    fn an_offset_no_place_uses_is_found_with_what_the_page_usually_has() {
        let (findings, census) = examine_posts(json!([
            { "ID": 1, "site_ID": 7, "date": "2026-11-17T15:26:58-05:00", "modified": "2026-11-17T15:26:58-05:00" },
            { "ID": 2, "site_ID": 7, "date": "2026-11-17T08:15:52-05:00" },
            { "ID": 3, "site_ID": 7, "date": "2026-11-16T16:45:34+91:00" },
            { "ID": 4, "site_ID": "7", "date": "2026-10-30T18:16:27-16:00" },
        ]));
        assert_eq!(census.posts, 4);
        assert_eq!(
            findings,
            vec![
                Finding {
                    kind: "odd_offset",
                    site_id: Some(7),
                    post_id: Some(3),
                    field: "date".into(),
                    value: "2026-11-16T16:45:34+91:00".into(),
                    usual_offset: Some("-05:00".into()),
                },
                Finding {
                    kind: "odd_offset",
                    site_id: Some(7),
                    post_id: Some(4),
                    field: "date".into(),
                    value: "2026-10-30T18:16:27-16:00".into(),
                    usual_offset: Some("-05:00".into()),
                },
            ]
        );
    }

    #[test]
    fn dates_that_say_nothing_are_not_findings() {
        let (findings, _) = examine_posts(json!([
            { "ID": 1, "date": "", "modified": false },
            { "ID": 2, "date": null },
            { "ID": 3 },
        ]));
        assert_eq!(findings, vec![]);
    }

    #[test]
    fn other_dates_that_are_not_right_are_found() {
        let (findings, _) = examine_posts(json!([
            { "ID": 1, "date": "yesterday" },
            { "ID": 2, "date": "1969-12-31T19:00:00-05:00" },
            { "ID": 3, "date": "2031-01-01T00:00:00+00:00" },
            { "ID": 4, "attachments": { "9": { "date": "2026-01-01T00:00:00-59:17" } } },
        ]));
        assert_eq!(
            kinds(&findings),
            vec![
                ("unreadable_date", "date", "yesterday"),
                ("odd_year", "date", "1969-12-31T19:00:00-05:00"),
                ("odd_year", "date", "2031-01-01T00:00:00+00:00"),
                (
                    "odd_offset",
                    "attachments.date",
                    "2026-01-01T00:00:00-59:17"
                ),
            ]
        );
    }

    #[test]
    fn a_count_or_an_id_below_zero_is_found() {
        let (findings, _) = examine_posts(json!([{
            "ID": 1,
            "like_count": -2,
            "featured_media": { "width": -340, "height": 200 },
            "author": { "ID": 5, "site_ID": -1 },
        }]));
        assert_eq!(
            kinds(&findings),
            vec![
                ("negative_number", "featured_media.width", "-340"),
                ("negative_number", "like_count", "-2"),
            ]
        );
    }

    #[test]
    fn the_census_counts_the_type_each_field_arrives_as() {
        let (_, mut census) = examine_posts(json!([
            { "ID": 1, "site_ID": "7", "like_count": "3", "tags": [], "author": { "ID": 5, "name": "" } },
            { "ID": 2, "site_ID": 7, "like_count": 3, "tags": { "a": {} }, "author": false },
        ]));
        let of = |census: &Census, field: &str| census.fields[field].clone();
        assert_eq!(
            of(&census, "site_ID"),
            BTreeMap::from([("number", 1), ("numeric string", 1)])
        );
        assert_eq!(
            of(&census, "like_count"),
            BTreeMap::from([("number", 1), ("numeric string", 1)])
        );
        assert_eq!(
            of(&census, "tags"),
            BTreeMap::from([("empty array", 1), ("object", 1)])
        );
        assert_eq!(
            of(&census, "author"),
            BTreeMap::from([("false", 1), ("object", 1)])
        );
        assert_eq!(
            of(&census, "author.name"),
            BTreeMap::from([("empty string", 1)])
        );

        let (_, more) = examine_posts(json!([{ "ID": 3, "site_ID": 7 }]));
        census.merge(more);
        assert_eq!(census.posts, 3);
        assert_eq!(
            of(&census, "site_ID"),
            BTreeMap::from([("number", 2), ("numeric string", 1)])
        );
    }

    #[test]
    fn the_posts_of_a_stream_of_cards_are_examined() {
        let body = json!({ "cards": [
            { "type": "post", "data": { "ID": 1, "site_ID": 7, "date": "2026-11-16T16:45:34+91:00" } },
            { "type": "recommended_blogs", "data": [{ "ID": 8, "date": "2026-11-16T16:45:34+91:00" }] },
            { "type": "post", "data": false },
        ]});
        let (findings, census) = examine(body.to_string().as_bytes(), 2026);
        assert_eq!(census.posts, 1);
        assert_eq!(
            kinds(&findings),
            vec![("odd_offset", "date", "2026-11-16T16:45:34+91:00")]
        );
    }

    #[test]
    fn a_response_without_posts_gives_nothing() {
        for body in [
            "not json",
            "[]",
            "{}",
            r#"{"posts": null}"#,
            r#"{"error": "unauthorized"}"#,
        ] {
            let (findings, census) = examine(body.as_bytes(), 2026);
            assert_eq!(findings, vec![]);
            assert_eq!(census, Census::default());
        }
    }
}
