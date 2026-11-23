# Syncing WordPress.com notifications

`NotificationService` keeps one WordPress.com account's notifications (notes) in the cache, and brings them up to date by comparing hashes. Use it instead of the endpoints in [notification-endpoints.md](notification-endpoints.md) when an app shows a list of notes that should work offline. Every request it sends is a GET: it never marks a note read or seen.

The most common task is showing the cached notes, then syncing the newest:

```swift
let service = try client.makeNotificationService(cache: cache, userID: account.userID)
service.setLanguage(language: .english)

let notes = try service.loadNotes(filter: NotificationsFilter())  // show these at once
let result = try await service.sync()                              // then bring them up to date
```

In Kotlin it's `client.makeNotificationService(cache, userId)`, and in Rust `NotificationService::new(delegate, cache, user_id)`. The cache's migrations must have run. Create one service per account and share it.

## Showing the notes

- `load_notes(filter)` returns the cached notes, newest first, each read from the JSON the API sent. A note with no date is last. `NotificationsFilter` picks unread notes or some types.
- `is_relevant_update(hook)` says whether a change to the cache is one the list shows. Call `load_notes` again when it's true, as with the Reader's collections.
- `info()` says how many notes are cached, how many the API lists and never sends whole, whether the oldest note has been reached, and when the last sync was.

## Syncing and paging

- `sync()` lists the newest 100 notes' IDs and hashes, downloads only the ones that are new or changed, and removes cached notes the server no longer lists. Call it when the list is shown, and when a push says a note arrived.
- `load_older()` loads the next older page the same way. Call it when the list scrolls to its end, until `reached_end` is true. After that it sends nothing.
- If more notes arrived since the last sync than a sync checks, `load_older` pages through the notes between them again, so none is missed.
- `WpApiCache::remove_wordpress_com_account` removes the account's notes. Call it when the user signs out.

## Traps

- **Set the language once, and keep it.** `set_language` sets the language every request is sent in, as its WordPress.com locale (`en`, `pt-br`). A request in a different language from the account's previous one changes every note's hash, so the sync after a change downloads every note again. `None` sends English. The delegate's `language_provider` isn't used.
- **Other clients make the service download notes again.** Fetching a note whole gives it a new hash. So when the web or another app fetches a note between two syncs, the next sync downloads it, though nothing changed. That's expected.
- **Some notes are never sent whole.** A list of IDs can have notes the API won't send. The service remembers each one with its hash, and asks again only when that hash changes. `info().listed_only_count` counts them.
- **Which notes the API sends can depend on the `User-Agent`.** The app's request executor sets it, not the service.
- **`load_older` can fail with `CannotPageFurther`.** That happens when more notes share one second, or more notes in a row can't be sent whole, than a page of 200 holds. Nothing changes, and calling it again fails the same way.
