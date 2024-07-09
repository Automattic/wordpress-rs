Unlike other mobile libraries where we create a GitHub release to release the library, we can't do that for this repository.

To publish a new release, we need to use the "New Build" button on [Buldkite](https://buildkite.com/automattic/wordpress-rs) to create a custom build with a `NEW_VERSION=<major.minor.patch>` environment variable.

Alternatively, you can run `make release-on-ci` command locally to trigger a release job on Buildkite, which requires a Buildkite API token with `write_builds` permission.
