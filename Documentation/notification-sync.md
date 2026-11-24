# Syncing WordPress.com notifications

`NotificationService` keeps one WordPress.com account's notifications (notes) in the cache, brings them up to date by comparing hashes, and marks them read, unread and seen. Use it instead of the endpoints in [notification-endpoints.md](notification-endpoints.md) when an app shows a list of notes that should work offline. A sync sends only GET requests: only the calls in "Marking notes" change the account.

The most common task is showing a tab's cached notes, then syncing its newest:

```swift
let service = try client.makeNotificationService(cache: cache, userID: account.userID)
service.setLanguage(language: .english)

let notes = try service.loadTabNotes(tab: .unread)   // show these at once
let result = try await service.syncTab(tab: .unread)  // then bring them up to date
```

In Kotlin it's `client.makeNotificationService(cache, userId)`, and in Rust `NotificationService::new(delegate, cache, user_id)`. The cache's migrations must have run. Create one service per account and share it.

## Tabs

`NotificationTab` has the five tabs of the web notifications panel, with the same notes in each:

| Tab | Shows |
|---|---|
| All | every note |
| Unread | notes not read; a note that doesn't say is unread |
| Comments | `comment` notes |
| Subscribers | `follow` notes |
| Likes | `like` and `comment_like` notes |

The WordPress apps agree, except that the Android app's Comments tab also has mentions of the user in a comment. Show a tab's name with `localize_notification_tab` (Swift: `tab.localizedText`, Kotlin: `tab.localizedDescription()`).

A note is stored once, however many tabs show it. A note one tab downloaded isn't downloaded again for another.

## Showing the notes

- `load_tab_notes(tab)` returns the cached notes a tab shows, newest first, each read from the JSON the API sent. A note with no date is last. `load_notes(filter)` does the same for any `NotificationsFilter`.
- `is_relevant_update(hook)` says whether a change to the cache is one the lists show. Call `load_tab_notes` again when it's true, as with the Reader's collections.
- `info()` says how many notes are cached, how many the API lists and never sends whole, whether the All tab reached the oldest note, and when the last sync was.

## Syncing and paging

- `sync_tab(tab)` lists the tab's newest 100 notes' IDs and hashes, downloads only the ones that are new or changed, and removes cached notes of the tab the server no longer lists. Call it when the tab is shown. `sync()` is the All tab.
- `load_older_in_tab(tab)` loads the tab's next older page the same way. Call it when the tab scrolls to its end, until `reached_end` is true. After that it sends nothing. Each tab keeps its own place, and `tab_reached_end(tab)` says whether it's at the end. `load_older()` is the All tab.
- If more notes arrived since the last sync than a sync checks, the older pages run again from the new notes, so none is missed.
- `WpApiCache::remove_wordpress_com_account` removes the account's notes. Call it when the user signs out.

## Marking notes

- `mark_read(ids)` and `mark_unread(ids)` mark notes read or unread. A note marked read leaves the Unread tab's list at once, and one marked unread joins it.
- `mark_all_read(tab, up_to)` marks every note of a tab at or before `up_to` read. Without `up_to`, it's the tab's newest cached note, so a note that arrives after the user asked stays unread. For the All and Unread tabs it's one request that covers notes the cache doesn't have too. Another tab marks its cached unread notes by ID, because the API can't mark every note of a type.
- `mark_seen(time)` records that the user has seen the notes up to `time`, or up to the newest cached note without one. Call it when the list is shown. It clears the bell, and can't be undone.
- `has_unseen_notes()` says from the cache whether a note is newer than the newest the user saw: whether to light the bell. `fetch_has_unseen_notes()` asks the server, which knows every note, as the WordPress iOS app does.

Each change shows in the cache at once, and observers hear of it through `is_relevant_update`. It's sent after that, and each call says what became of it:

| Outcome | What happened | What the user sees |
|---|---|---|
| `Sent` | The server has it. | The change |
| `Waiting` | No answer came: the device is offline, the request timed out, the server failed, or the server rejected the sign-in. The change is kept, across launches. | The change |
| `Unchanged` | Nothing to send: the notes already show that way, or the seen time isn't newer. | No change |
| An error | The server refused it for good: an answer in the 400s other than 401, 408 or 429, or a success that says it didn't make the change (`ChangeNotMade`). The change is dropped. | The notes as the server has them |

Changes that wait are sent by `send_pending_changes()`, which returns how many still wait, and by the next change of any kind. Call it at launch, when the app comes to the foreground, and when the device is back online. `pending_change_count()` counts them. A change refused when it's sent later is logged, and the notes go back to the server's state.

## When the app hears of a change

An app with its own push handling or socket passes the note's ID to the service:

- `refresh_note(id)` fetches that one note and stores it. A note that doesn't come is removed from the cache: it was deleted, or the API doesn't send it whole, and the next sync finds out which.
- `remove_note(id)` removes a deleted note. It sends no request.

Both change the cache, so `is_relevant_update` says to reload. With no push or socket, call `sync()` on a timer: when nothing changed, that's one request, for IDs and hashes only.

## Traps

- **A sync doesn't undo a change that waits.** Until a change reaches the server, the server still has the old state, and a sync stores it. The cached lists show the waiting change over it. Changes and syncs take turns, so a change is sent after a sync that's running, and the next sync sees the server after it.
- **The seen time only moves forward.** `mark_seen` ignores a time that isn't after the newest the service knows, and a seen time that waits is dropped, unsent, if a sync brings a newer one from another device.
- **Marking a note gives it a new hash.** The next sync downloads it again, as it does a note another client fetched.
- **Set the language once, and keep it.** `set_language` sets the language every request is sent in, changes included, as its WordPress.com locale (`en`, `pt-br`). A request in a different language from the account's previous one changes every note's hash, so the sync after a change downloads every note again. `None` sends English. The delegate's `language_provider` isn't used.
- **Other clients make the service download notes again.** Fetching a note whole gives it a new hash. So when the web or another app fetches a note between two syncs, the next sync downloads it, though nothing changed. That's expected.
- **The Unread tab never removes a note.** A note missing from its list may only have been read. The service downloads it again, which brings whether it was read. One that doesn't come stays until the All tab's sync, which lists every note.
- **Some notes are never sent whole.** A list of IDs can have notes the API won't send. The service remembers each one with its hash, and asks again only when that hash changes. `info().listed_only_count` counts them.
- **Which notes the API sends can depend on the `User-Agent`.** The app's request executor sets it, not the service.
- **Loading older notes can fail with `CannotPageFurther`.** That happens when more notes share one second, or more notes in a row can't be sent whole, than a page of 200 holds. Nothing changes, and calling it again fails the same way.
