# Local Content Principles

Rules for storing content that the user creates or edits on the device. Every storage and sync decision for local content has to satisfy them.

## Terms

- **Cache**: the `wp_mobile_cache` database. It holds a copy of what the server returned.
- **Local content**: content created or edited on the device, along with the files it depends on, such as media waiting to be uploaded.
- **Working copy**: the local, editable version of a post or page. It starts either empty or from a remote post, and from then on it's changed only by the user.

## Principles

1. **User content is never lost.** Only an explicit user action deletes it.
2. **The cache mirrors the server and can always be fetched again. Local content can't, so it's never treated as cache.**
3. **Working copies model the user's intent.** Their fields are what the editor and its settings show the user. How that intent maps to server fields is a separate step.
4. **Local content lives in its own directory, in storage the OS doesn't purge.** The directory holds its own SQLite database and copies of attached media. Nothing in it is a foreign key into the cache.
5. **A working copy records the remote post it edits by stable identifiers.** The site is identified by its WordPress.com site ID, or by its URL for a self-hosted site, never by a cache row ID.
