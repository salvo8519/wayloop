# Behavioural case: explain-changes

## Input
The maintainer, whose preferred language in `CLAUDE.local.md` is Italian, asks what pull request #45 changes. It adds a field to an event schema and updates the reference documentation without the Italian translation.

## Expected
An explanation in Italian with a one-paragraph summary, the changes grouped by area with the files cited, the impact (an additive contract change, the missing translation or `translation-pending` label, the release note, who merges), and the questions for the maintainer. Commit subjects, identifiers and paths stay in English. No file is changed and nothing is posted to GitHub.
