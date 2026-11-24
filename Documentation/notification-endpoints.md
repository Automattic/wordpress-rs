# WordPress.com notification endpoints

Every WordPress.com notification endpoint the library has: what it's for, the parameters it sends, what comes back, and the traps a caller must know. `WPCOM_REST_API_CHECKLIST.md` lists the notification endpoints that aren't built yet. To keep an account's notes in the cache and sync them, use `NotificationService`: see [notification-sync.md](notification-sync.md).

The most common task is fetching the newest page of notes with their note context, and showing each one:

```rust
let page = client
    .notifications()
    .list(&NotificationsParams {
        query: NotificationsQuery {
            number: Some(50),
            ..Default::default()
        },
        include_body: true,
        include_context: true,
    })
    .await?
    .data;
for note in &page.notes {
    let layout = notification_layout(note);
    // Draw `layout`. Show a `NotificationLabel` with `localize_notification_label`.
}
```

In Swift the endpoints are `WPComApiClient.notifications`, a note's layout is `WpComNotification.layout`, and a label's text is `NotificationLabel.localizedText`. In Kotlin they are `client.request { it.notifications().list(params) }`, `notificationLayout(note)` and `label.localizedDescription()`.

Every request needs a token. The endpoints are in `wp_api/src/wp_com/endpoint/notifications_endpoint.rs`, and the note model is in `wp_api/src/wp_com/notifications.rs`.

## Traps

- **A short page is not the end.** A page can have fewer notes than asked for, or none, while older notes exist. The API leaves out notes it can't show, and a page that left some out can be short. To page back, send the oldest note's time as `before`.
- **A list of IDs or hashes has notes a list of whole notes leaves out.** The notes the API can't show are left out of whole notes, but `list_ids` and `list_hashes` still list them. Take an ID to fetch from a list of whole notes.
- **`before` includes its second**, so the oldest note of one page comes back at the top of the next.
- **Hashes only compare between requests made the same way.** A note's hash changes when anything in it changes, including that it was read. It also differs between requests that ask for different fields, and between requests in different languages.
- **Send one fixed language.** A request in a different `locale` from the one before it changes every note's hash, so send the same `locale` on every request.
- **A note that doesn't exist is an empty success.** Fetching a note that doesn't exist, or that the user can't see, returns a page with no notes, not an error.
- **A time at the very start of 1970 means no date.** Some notes have no date of their own, and come with a time on 1 January 1970.
- **Positions count UTF-16 units.** A range's or medium's `indices` count UTF-16 units, as a Kotlin `String` and an `NSString` do. Ranges can nest, overlap and cover no text, and some end past their text. `notification_text_styled_runs` and `notification_block_styled_runs` turn them into runs that fit.
- **Note context is asked for, not sent.** Without `include_context`, a note's `meta` has no actor, summary, post, site or parent comment, and every note's layout is a notice.
- **A note that can't be read doesn't fail its page.** It's listed in the page's `unreadable_notes`.
- **A write's parameters all go in the body.** The API reads `counts`, `time` and `source` from the body only, and `read/all` refuses a `time` in the address. The client's language, if it has one, still goes in the address.
- **Marking every note read can't be narrowed to a type.** `read/all` takes a time and nothing else. To mark one tab's notes, mark them by ID.
- **A success isn't always the change.** Check `success`, and for `read`, which notes `updated` lists as unread afterwards.

## `GET /rest/v1.1/notifications`

A page of the user's notes, newest first. The library has three requests for it, which differ in what they send of each note:

| Request | `fields` sent | Response |
|---|---|---|
| `NotificationsRequest::List` | the whole note, with or without its header and body, and with or without its note context | `NotificationsResponse` |
| `NotificationsRequest::ListWithJson` | as `List` | `NotificationsWithJsonResponse`: each note with its JSON as the API sent it, for a cache to keep |
| `NotificationsRequest::ListHashes` | `id,note_hash` | `NotificationHashesResponse` |
| `NotificationsRequest::ListIds` | `id` | `NotificationIdsResponse` |

Parameters the library sends, in the query (`NotificationsQuery`):

- `number` (`u32`): How many notes to return: 200 at most.
- `before` (`WpGmtDateTime`): Only notes from this second or earlier, sent as seconds since 1970.
- `since` (`WpGmtDateTime`): Only notes from this time on, sent as seconds since 1970.
- `ids` (`Vec<NotificationId>`): Only these notes, comma-separated.
- `type` (`Vec<NotificationType>`): Only notes of these types, comma-separated.
- `unread` (`bool`): Only notes the user hasn't read. Sent as `1`, and only when set.
- `read` (`bool`): Only notes the user has read. Sent as `1`, and only when set.
- `locale` (`WPComLanguage`): The language to write the notes in.
- `source` (`String`): A name for the app making the request.
- `fields` (`String`): What to send of each note. `NotificationsParams` sets it for whole notes:
  - `include_body` adds `header,body`. Without them a page is much smaller.
  - `include_context` adds `note_context`, which isn't a field: it turns on the note context in each note's `meta`.
  - `note_hash` is always asked for. Like `note_context`, it's sent only when asked for.

What comes back: the notes, the notes that couldn't be read, and `last_seen_time`, when the user last looked at their notifications. The API sends that time as a string of seconds since 1970.

Public documentation: not listed in WordPress.com's REST API reference.

## `GET /rest/v1.1/notifications/$note_id`

One note, as a page that has it or has no notes.

Request `NotificationsRequest::Get`, response `NotificationsResponse`.

Parameters the library sends, in the query (`NotificationParams`):

- `locale` (`WPComLanguage`): As for the list.
- `source` (`String`): As for the list.
- `fields` (`String`): As for the list's whole notes, from `include_body` and `include_context`.

Public documentation: not listed in WordPress.com's REST API reference.

## `POST /rest/v1.1/notifications/read`

Marks notes read or unread. A note's read state is a count of unread updates the server keeps: a positive amount lowers it, down to read, and a negative amount raises it, which makes the note unread.

Request `NotificationsRequest::MarkRead`, response `NotificationsReadResponse`.

Parameters the library sends, in the body (`NotificationsReadParams`):

- `counts` (from `changes: Vec<NotificationReadChange>`): Each note's ID, with `9999` to mark it read, as the WordPress apps send, or `-1` to mark it unread, as the web panel sends. Read and unread can be mixed.
- `source` (`String`): Which kind of client is asking: `mobile`, `notifications-panel` or `reader-conversations`. Used for the API's stats only.

What comes back: `success`, and `updated`, the notes of the request that are unread afterwards.

Public documentation: [`POST /notifications/read`](https://developer.wordpress.com/docs/api/1/post/notifications/read/).

## `POST /rest/v1.1/notifications/read/all`

Marks every note at or before a time read, without listing them.

Request `NotificationsRequest::MarkAllRead`, response `NotificationsReadAllResponse`.

Parameters the library sends, in the body (`NotificationsReadAllParams`):

- `time` (`WpGmtDateTime`): Only notes at or before this second, sent as seconds since 1970. Leaving it out marks every note read. Send the newest note the user saw, so one that arrived since stays unread.
- `source` (`String`): As for `read`.

What comes back: `success`, `count`, how many notes were marked, and `last_seen_time`.

Public documentation: [`POST /notifications/read/all`](https://developer.wordpress.com/docs/api/1/post/notifications/read/all/).

## `POST /rest/v1.1/notifications/seen`

Records the time of the newest note the user has seen. It clears the unseen flag, and can't be undone.

Request `NotificationsRequest::MarkSeen`, response `NotificationsSeenResponse`.

Parameters the library sends, in the body (`NotificationsSeenParams`):

- `time` (`WpGmtDateTime`): Sent as seconds since 1970. The API reads any other form as a number, and answers success without moving the time.
- `source` (`String`): As for `read`.

What comes back: `success`, and `last_seen_time` as the server now has it.

Public documentation: [`POST /notifications/seen`](https://developer.wordpress.com/docs/api/1/post/notifications/seen/).

## `GET /rest/v1.1/me?fields=has_unseen_notes`

Whether the account has a note newer than the last seen time: the flag that lights the notifications bell. Request `MeRequest::UnseenNotes`, response `WpComUnseenNotes`. A missing flag is an error, not a clear bell.

## Showing a note

`notification_layout` builds a `NotificationLayout` from a note: its title, a line to show in a list, and its content in one of five layouts, as WordPress.com's web notifications panel shows it. A note needs its note context to get one of the four special layouts:

| Layout | Note types |
|---|---|
| Comment | `comment`, and `automattcher` for a mention in a comment |
| Reactions | `like`, `comment_like`, `reblog` |
| Follow | `follow`, `activitypub_followed_note`, `activitypub_interaction_note` |
| New post | `new_post`, and `automattcher` for a mention in a post |
| Notice | every other type, and every note without note context |

Text the API wrote comes through as it was sent, as styled runs where it has ranges. A label the library writes, such as "12 likes on your post", is a `NotificationLabel`, in the web panel's words. `localize_notification_label` returns it in the user's language, or in English until it's translated.

## Testing against WordPress.com

- **Live trials:** `wp_com_e2e/src/notifications_tests.rs` checks the GET endpoints against the account of `WP_COM_API_KEY`, and `notification_service_tests.rs` syncs and pages the account's notes with `NotificationService`. Every request is a GET, so nothing is marked read or seen. Run both with `cargo run -q -p wp_com_e2e --bin wp_com_e2e -- notification`.
- **Live trials that write:** `notification_write_tests.rs` marks notes read, unread and seen on the account, and puts back what it can. Each is ignored, so the command above never runs one. Run one by its exact name, after reading its doc comment: see the module comment there.
- **A whole account:** `notifications_sweep` walks every note of an account and reports any note the library can't read, can't turn into styled runs or can't lay out. `--report-only` writes only counts and type names, so someone can run it on their own account and send back the report. Its module comment, in `wp_com_e2e/src/bin/notifications_sweep/main.rs`, says how to run it.
