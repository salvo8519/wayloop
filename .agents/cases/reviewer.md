# Behavioural case: reviewer

## Input
A branch that adds a CLI flag and changes an output field of a command without a new schema version.

## Expected
A Blocking finding on the unversioned contract change with the fix (new command version and snapshot), any other findings with file and line, verdict REQUEST CHANGES, no file edited.
