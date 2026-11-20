# Contract cases

Golden contract cases of the engine commands (ADR 0009 C5, plan of SPEC-0001 D6): a fixed
input sent to a command and the result an agent can rely on. A case is part of the contract:
once it is on `main` it is never edited, renamed or deleted; new behaviour gets a new case, and
a released case that is wrong is a question for the maintainer and, if confirmed, a new command
version with new cases.

Layout: `tests/contract/<command>/v<N>/<case>.json`, one file per case. Expected values come
from the specification and the ADRs, never from running the code under test.

## Format

| Member | Meaning |
| --- | --- |
| `description` | What the case shows, in one or two sentences |
| `covers` | The requirements and acceptance criteria it covers, for example `SPEC-0001 R16` |
| `context` | Optional: the settings of the channel passed to the entry point, today `{"command_version": <n>}`; the command line passes it as `--command-version <n>` |
| `input` | The canonical input, as a JSON value; or |
| `input_text` | The input as text, for inputs a JSON value cannot hold (a repeated key, a number beyond a double, text that is not JSON) |
| `expect.exit_code` | The exit code of the envelope (ADR 0009 C7) |
| `expect.values` | Values of the envelope, keyed by their RFC 6901 pointer |
| `expect.absent` | Pointers that must not resolve in the envelope |
| `expect.lengths` | Lengths of lists of the envelope, keyed by their pointer, so that no extra entry passes unnoticed |

A case does not assert `meta.engine_version`, the wording of `message`, or the lists
`diagnostics.checks` and `diagnostics.not_done`, which change as the checks of the engine are
built.

A case with `input` asserts `/meta/input_hash`. Its value was computed apart from the engine,
with Python 3: `json.dumps` of the input without its members set to null, with
`sort_keys=True`, `separators=(",", ":")` and `ensure_ascii=False`, then `hashlib.sha256`.
For the inputs of these cases (ASCII keys, integers only) that text is the canonical JSON of
RFC 8785. The runner recomputes it with a second implementation of its own (canonical JSON
written in the test, SHA-256 of the `sha2` crate), so that a case with a float or a non-ASCII
key cannot pass unnoticed.

A case with `input_text` asserts `/meta/input_hash` too: the engine refuses such text before
it reads it as JSON, so the hash is the SHA-256 of the bytes received (SPEC-0004 R11),
computed with `hashlib.sha256` of the text in UTF-8 and recomputed by the runner.

## Runner

`crates/wayloop-cli/tests/contract.rs` sends every case to the entry point of the core and to
the built binary, through a file and through standard input. It checks the expectation on the
envelope of the entry point; the envelope against the open output snapshot of the command and
against the schema of the same type generated for the input direction, where every
object is closed, so that the engine writes no member its contract does not describe (a test,
not a contract, with no file of its own); each envelope read back into its type and each
accepted input, without its members set to null, into `ValidateInput` (ADR 0030 G1); the
input hash of every case against its own computation;
and that the binary prints that envelope byte for byte and exits with its code. It also
checks that a command line not understood (exit code 2) and an input file that cannot be read
(exit code 3) print an envelope.
