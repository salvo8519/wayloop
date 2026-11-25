"""The rule of the file names of ADRs, for the checks of the documentation.

Standard library only. A deliberate copy of the rule of the engine,
`locations::adr_numbers` in `crates/wayloop-core/src/artifacts/locations.rs` (SPEC-0001 Q5,
#344), because the scripts cannot call the engine. The cases of
`tests/process/adr_names.json` are read by a test of the runtime, through the core, and by
`tests/process/test_adr.py`, so that the two copies stay equal.

A file of an ADR is `<number>-<slug>.md` directly in `FOLDER`: the number is a sequence from
`0001` (four digits, or five or more without a leading zero, as in an identifier) that fits
an unsigned 64-bit integer, and the slug has at least one character.
"""

from __future__ import annotations

import re
from collections.abc import Iterable

# The folder of the ADRs, relative to the root of the project (`locations::ADR_FOLDER`).
FOLDER = "docs/en/project/adr"
# The index of the folder, which is not an ADR.
INDEX = "index.md"
# The number of an ADR: the sequence of an identifier (`format::is_sequence` of the core).
NUMBER = r"[0-9]{4}|[1-9][0-9]{4,}"
NAME = re.compile(rf"(?P<number>{NUMBER})-[^/]+\.md")
# The largest number of an ADR (`AdrNumber` holds a non-zero unsigned 64-bit integer).
MAX_NUMBER = 2**64 - 1


def adr_number(name: str) -> int | None:
    """The number of the ADR whose file is named `name`, or None when it is not an ADR."""
    match = NAME.fullmatch(name)
    if match is None:
        return None
    number = int(match["number"])
    return number if 1 <= number <= MAX_NUMBER else None


def adr_numbers(names: Iterable[str]) -> list[int]:
    """The numbers of the ADRs among the file `names`, once each, in ascending order."""
    return sorted({n for n in map(adr_number, names) if n is not None})
