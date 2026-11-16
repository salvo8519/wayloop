# Release notes fragments

Every pull request that changes what users can do or get adds one short file here. At release
time `towncrier` turns them into `RELEASE_NOTES.md`.

Name: `<issue number>.<type>.md`, for example `12.feature.md`. Types: `breaking`, `feature`,
`improvement`, `fix`.

Write for an analyst or a project lead, not only for a developer: what changed for them and
why it matters, in one to three sentences, in English.

Changes with no effect for users (tests, CI, refactoring) need no fragment; label the pull
request `no-release-note`.
