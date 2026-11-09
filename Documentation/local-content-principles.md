# Local Content Principles

Rules for storing content that the user creates or edits on the device. Every storage and sync decision for local content has to satisfy them.

## Terms

- **Cache**: the `wp_mobile_cache` database. It's what we know the server has. It holds a copy of what the server returned when we last fetched it.
- **Local content**: content created or edited on the device, along with the files it depends on, such as media waiting to be uploaded.
- **Working copy**: the local, editable version of a post or page. It starts either as a new post or from a remote post, and from then on only the user changes its fields.

## Principles

1. **User content is never lost.** Only an explicit user action deletes it.
2. **The cache mirrors the server and can always be fetched again. Local content can't, so it's never treated as cache.**
3. **What the user has and what the server has are kept apart, and compared.** The working copy is what the user has. The cache is what the server had when we last fetched it. Whether a working copy has changes to push, or conflicts with the server, comes from comparing the two field by field. It's never stored as a flag or a status.
4. **Only wordpress-rs's own requests fill the cache.** When an editor or anything else writes to the server, wordpress-rs fetches what changed. It doesn't take the other component's copy of the result.
5. **Working copies model the user's intent.** Their fields are what the editor and its settings show the user. How that intent maps to server fields is a separate step.
6. **Local content lives in its own directory, in storage the OS doesn't purge.** The directory holds its own SQLite database and copies of attached media. Nothing in it is a foreign key into the cache.
7. **A working copy refers to its site and its remote post by stable identifiers.** The site is identified by its WordPress.com site ID when it has one, and otherwise by its URL, never by a cache row ID.
