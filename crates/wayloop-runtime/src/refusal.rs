//! What the runtime reports for a file it does not load: the line of the fault and a message
//! for a person, which the core reports as a `syntax` error of the file (SPEC-0001 R12).

use core::num::NonZeroU64;

use wayloop_schemas::validate::ParseError;
use wayloop_schemas::value::Line;

/// Most characters of user text quoted in a message; a longer text is cut and ends with `...`.
const QUOTED_CHARS: usize = 80;

/// The line `n`, counted from 1; line 1 for 0, which a parser gives for no position.
pub(crate) fn line(n: usize) -> Line {
    let n = u64::try_from(n).unwrap_or(u64::MAX);
    Line::new(NonZeroU64::new(n).unwrap_or(NonZeroU64::MIN))
}

/// A refusal at line `at`, with `message` for a person.
pub(crate) fn fault(at: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line: line(at),
        message: message.into(),
    }
}

/// `text`, written by a user, as a message quotes it: at most [`QUOTED_CHARS`] characters,
/// then `...` when it is longer.
pub(crate) fn quoted(text: &str) -> String {
    match text.char_indices().nth(QUOTED_CHARS) {
        Some((end, _)) => format!("{}...", text.get(..end).unwrap_or_default()),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{QUOTED_CHARS, fault, line, quoted};

    #[test]
    fn a_line_is_counted_from_one() {
        assert_eq!(line(0).get().get(), 1);
        assert_eq!(line(7).get().get(), 7);
        let refusal = fault(3, "why");
        assert_eq!(
            (refusal.line.get().get(), refusal.message.as_str()),
            (3, "why")
        );
    }

    #[test]
    fn a_long_text_is_cut_on_a_character() {
        let exact = "é".repeat(QUOTED_CHARS);
        assert_eq!(quoted(&exact), exact);
        let longer = format!("{exact}x");
        assert_eq!(quoted(&longer), format!("{exact}..."));
        assert_eq!(quoted(""), "");
    }
}
