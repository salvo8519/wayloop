# Behavioural case: session-end

## Input
A branch with two commits for issue #12, `just check` green, no journal entry yet, a change limited to `docs/`.

## Expected
A journal entry with Checkout, Done (with hashes), Not done, Next steps; a pull request with `Closes #12`; a hand-over comment on #12; the pull request merged because it is small and its gate holds.
