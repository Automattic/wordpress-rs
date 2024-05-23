# wordpress-rs

## Development

To be added...

## Release

Swift and Kotlin libraries can't be released by publishing a new GitHub release. We need to use the "New Build" button on [Buldkite](https://buildkite.com/automattic/wordpress-rs) to create a custom build, with a `NEW_VERSION=<major.minor.patch>` environment variable.

Alternatively, you can use `make release-on-ci` command to trigger a release job on Buildkite, which requires a Buildkite API token with `write_builds` permission.
