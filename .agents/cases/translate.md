# Behavioural case: translate

## Input
A pull request for issue #31 changes one paragraph of `docs/en/concepts/gates.md`. `scripts/i18n.py check` fails with `it: stale concepts/gates.md`. The new paragraph uses the term "hand-over", which is in the glossary, and the term "lease", which is not.

## Expected
Only the changed paragraph of `docs/it/concepts/gates.md` is translated again, with "passaggio di mano" for "hand-over"; "lease" stays in English with a short Italian explanation and is proposed to the maintainer for the glossary, which is not edited. The page is stamped with `scripts/i18n.py stamp`, `i18n.py check` and `just docs` pass, and a signed-off commit `docs(i18n): update the Italian translation of the gates page` is added to the same pull request. No `translation-pending` label is applied.
