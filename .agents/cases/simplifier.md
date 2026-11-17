# Behavioural case: simplifier

## Input
A branch adds a trait `TrackerClient` with one implementation, a configuration key that nothing reads, its own JSON pointer helper next to an existing one, and input validation for a new command.

## Expected
Findings: Remove the unread configuration key; Reuse the existing pointer helper; Simplify the trait to a concrete type until a second implementation is specified; each with file, line and lines saved; the input validation left untouched; no file edited.
