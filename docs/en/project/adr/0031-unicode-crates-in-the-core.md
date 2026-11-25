# ADR 0031: Unicode normalisation and case folding in the core with two unicode-rs crates

Status: Proposed · 2026-11-25 (milestone M0 · Foundations; decision tracked in #266)

## Context

SPEC-0001 R8 refuses two epic or task titles that are equal after Unicode NFC normalisation,
case folding, trimming and collapsing white space, and R11 builds the slug of a branch by
canonical decomposition, dropping combining marks. Both rules are decision logic, so they live
in `wayloop-core`, which performs no I/O and may depend only on the crates of its reviewed
allowlist (ADR 0001, ADR 0006, `scripts/check_core_io.py`). The dependency policy
(`docs/en/project/dependencies.md`) requires a record for every new runtime dependency of the
core that names its maintainer, its licence, its scope, the absence of I/O and a fallback.

The standard library has neither normalisation nor full case folding: `str::to_lowercase` is a
lower-casing, which differs from case folding for some characters (the sharp s `ß` folds to
`ss`, the final sigma to sigma). The plan of SPEC-0001 listed the options, decided by the
maintainer on 2026-11-19 (D4):

- **Normalisation.** (a) A normalisation crate on the allowlist. (b) Tables generated into the
  core by a script from the Unicode Character Database. Chosen: (a). Option (b) keeps the
  allowlist smaller but adds a generator, generated tables and their own update procedure to
  maintain, for an algorithm that a mature crate already implements.
- **Case folding.** (i) A case-folding crate on the allowlist. (ii) Lower-casing after NFC, with
  an edit of SPEC-0001 to say so. Chosen: (i), so that the rule is the Unicode one.

## Decision

1. `wayloop-core` depends on `unicode-normalization` (from 0.1.26) for NFC and canonical
   decomposition, and on `caseless` (from 0.2.2) for full case folding, each without default
   features. `caseless` enables the `std` feature of `unicode-normalization`, which adds no
   crate and no I/O.
2. The allowlist of `wayloop-core` in `scripts/check_core_io.py` gains `unicode-normalization`,
   `caseless` and `tinyvec`, the one crate they reach. The set is fixed by `cargo tree`; any
   other crate they come to reach fails the check until it is reviewed.
3. The facts reviewed on 2026-11-25:

   | Crate | Maintainer | Licence | Latest release | Scope | Unsafe code |
   | --- | --- | --- | --- | --- | --- |
   | `unicode-normalization` 0.1.26 | The unicode-rs project | (MIT OR Apache-2.0) AND Unicode-3.0 | 2026-10-16 | Normalisation forms of UAX 15, tables of Unicode 18.0.0 | Confined: the composition of Hangul syllables and characters built from table values |
   | `caseless` 0.2.2 | The unicode-rs project (author Simon Sapin) | MIT | 2024-12-30 | Default and canonical caseless matching and folding, tables of Unicode 16.0.0 | None |
   | `tinyvec` 1.13.3 | Lokathor | Zlib OR Apache-2.0 OR MIT | 2026-09-13 | Inline vectors, the buffer of `unicode-normalization` | Forbidden by the crate (`forbid(unsafe_code)`) |

   None has a build script, reads files, the environment or the clock, or opens a connection.
   Every licence is on the allow-lists of `deny.toml` and of the Security workflow, and every
   release is older than the cooldown of 7 days.
4. Only the naming rules call these crates (`crates/wayloop-core/src/naming/`). A rule that
   needs them names the Unicode operation it applies, so that a change of crate keeps the rule.

## Consequences

- R8 and R11 follow Unicode, not an approximation, and the core keeps no Unicode table of its
  own.
- Two risks are accepted and watched:
  - `caseless` had no release in the 23 months before this record. Its folding table is
    complete for Unicode 16.0.0 and the folding of a character rarely changes between versions,
    but a new version of Unicode may add folds that it does not know.
  - The two crates use tables of two versions of Unicode, 16.0.0 for folding and 18.0.0 for
    normalisation. A character added after Unicode 16.0.0 is normalised but not folded, so two
    titles that differ only in the case of such a character are not found equal.
- Fallback if a crate is abandoned or a vulnerability is not fixed: option (b) of D4, the tables
  generated into the core by a script from the Unicode Character Database, behind the same
  functions of `naming`, with the tests of R8 and R11 unchanged.
- Updates of these crates are reviewed, never merged automatically, like every dependency of
  the core. A new release of `caseless`, or a replacement crate, is a reason to review this
  record.
- The security review of the three crates follows the commit that adds them (#266).
