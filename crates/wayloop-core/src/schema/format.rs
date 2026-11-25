//! Named formats of the subset and the fixes derived from them (SPEC-0001, Identifiers and
//! the field tables; the list is in the documentation of the parent module).
//!
//! Every check works on ASCII and reads the text once; no regular expression is involved. A
//! fix is offered only when it is a value the format accepts and differs from the text.

use wayloop_schemas::{Format, Pointer};

/// How a named format is checked. The match in [`shape`] is exhaustive over [`Format`], the
/// one list of named formats shared with the generator of `wayloop-schemas` (ADR 0030 G2): a
/// format added there does not compile until it is checked here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// RFC 3339 `full-date`.
    Date,
    /// RFC 3339 date-time in UTC, written with `Z`.
    UtcDateTime,
    /// BCP 47 language tag: language, script, region and variants.
    LanguageTag,
    /// Tracker handle, `@` and a user name.
    TrackerHandle,
    /// A path relative to the root of the project, in one spelling.
    ProjectPath,
    /// Lower-case words of ASCII letters and digits joined by single hyphens.
    HyphenatedWords,
    /// A name of the policy: a lower-case ASCII letter, then lower-case ASCII letters, digits and
    /// hyphens.
    PolicyName,
    /// `<PREFIX>-<seq>`, with one of the prefixes: four digits, or five or more without a
    /// leading zero.
    Sequence(&'static [&'static str]),
    /// `M<n>`: `0` or a number without a leading zero.
    Milestone,
    /// `HANDOVER-<issue>-<k>`, both numbers from 1.
    Handover,
    /// `<PREFIX><k>`, a part of an artifact: a number from 1.
    Part(&'static str),
    /// An RFC 6901 JSON pointer, as [`Pointer::parse`] reads it.
    JsonPointer,
}

/// The shape that checks `format`.
const fn shape(format: Format) -> Shape {
    match format {
        Format::Date => Shape::Date,
        Format::UtcDateTime => Shape::UtcDateTime,
        Format::LanguageTag => Shape::LanguageTag,
        Format::TrackerHandle => Shape::TrackerHandle,
        Format::ProjectPath => Shape::ProjectPath,
        Format::Capability => Shape::HyphenatedWords,
        Format::PolicyName => Shape::PolicyName,
        Format::BriefId => Shape::Sequence(&["BRIEF"]),
        Format::CouncilId => Shape::Sequence(&["COUNCIL"]),
        Format::BriefOrCouncilId => Shape::Sequence(&["BRIEF", "COUNCIL"]),
        Format::GateId => Shape::Sequence(&["GATE"]),
        Format::EpicId => Shape::Sequence(&["EPIC"]),
        Format::StoryId => Shape::Sequence(&["STORY"]),
        Format::SpecId => Shape::Sequence(&["SPEC"]),
        Format::ChangeId => Shape::Sequence(&["CHANGE"]),
        Format::TaskId => Shape::Sequence(&["TASK"]),
        Format::MilestoneId => Shape::Milestone,
        Format::HandoverId => Shape::Handover,
        Format::RequirementId => Shape::Part("R"),
        Format::CriterionId => Shape::Part("AC"),
        Format::DecisionId => Shape::Part("D"),
        Format::QuestionId => Shape::Part("Q"),
        Format::JsonPointer => Shape::JsonPointer,
    }
}

/// Whether `text` is in the form of `format`.
pub(crate) fn accepts(format: Format, text: &str) -> bool {
    shape(format).accepts(text)
}

/// The prefix of `format` when it is the identifier of a part of an artifact, such as `R` of a
/// requirement; `None` for any other format.
pub(crate) const fn part_prefix(format: Format) -> Option<&'static str> {
    if let Shape::Part(prefix) = shape(format) {
        Some(prefix)
    } else {
        None
    }
}

/// A corrected value for `text`, when one can be derived and `format` accepts it.
pub(crate) fn fix(format: Format, text: &str) -> Option<String> {
    shape(format).fix(text)
}

impl Shape {
    /// Whether `text` is in the form of the shape.
    fn accepts(self, text: &str) -> bool {
        match self {
            Shape::Date => is_date(text.as_bytes()),
            Shape::UtcDateTime => is_utc_date_time(text.as_bytes()),
            Shape::LanguageTag => is_language_tag(text),
            Shape::TrackerHandle => is_tracker_handle(text),
            Shape::ProjectPath => is_project_path(text),
            Shape::HyphenatedWords => is_hyphenated_words(text),
            Shape::PolicyName => is_policy_name(text),
            Shape::Sequence(prefixes) => prefixes.iter().any(|prefix| {
                text.strip_prefix(prefix)
                    .and_then(|rest| rest.strip_prefix('-'))
                    .is_some_and(is_sequence)
            }),
            Shape::Milestone => text
                .strip_prefix('M')
                .is_some_and(|n| n == "0" || is_counter(n)),
            Shape::Handover => text
                .strip_prefix("HANDOVER-")
                .and_then(|rest| rest.split_once('-'))
                .is_some_and(|(issue, k)| is_counter(issue) && is_counter(k)),
            Shape::Part(prefix) => text.strip_prefix(prefix).is_some_and(is_counter),
            Shape::JsonPointer => Pointer::parse(text).is_some(),
        }
    }

    /// A corrected value for `text`, when one can be derived and the shape accepts it.
    fn fix(self, text: &str) -> Option<String> {
        let candidate = match self {
            Shape::Date | Shape::UtcDateTime | Shape::JsonPointer => None,
            Shape::LanguageTag => Some(text.replace('_', "-")),
            Shape::TrackerHandle => (!text.starts_with('@')).then(|| format!("@{text}")),
            Shape::ProjectPath => project_path_fix(text),
            Shape::HyphenatedWords => hyphenated_words_fix(text),
            Shape::PolicyName => Some(text.to_ascii_lowercase()),
            Shape::Sequence(prefixes) => prefixes.iter().find_map(|prefix| {
                after_prefix(text, &[prefix])
                    .and_then(number)
                    .map(|n| format!("{prefix}-{n:0>4}"))
            }),
            Shape::Milestone => after_prefix(text, &["MILESTONE", "M"])
                .and_then(number)
                .map(|n| format!("M{n}")),
            Shape::Handover => after_prefix(text, &["HANDOVER"]).and_then(|rest| {
                let (issue, k) = rest.split_once('-')?;
                Some(format!("HANDOVER-{}-{}", number(issue)?, number(k)?))
            }),
            Shape::Part(prefix) => after_prefix(text, &[prefix])
                .and_then(number)
                .map(|n| format!("{prefix}{n}")),
        }?;
        (candidate != text && self.accepts(&candidate)).then_some(candidate)
    }
}

/// Whether `text` is a path relative to the root of the project in its one spelling: segments
/// joined by `/`, none empty, `.` or `..`; no `\`; no drive letter (`C:`) at the start.
fn is_project_path(text: &str) -> bool {
    let drive = matches!(text.as_bytes(), [letter, b':', ..] if letter.is_ascii_alphabetic());
    !drive
        && !text.contains('\\')
        && text
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}

/// The path written in its one spelling: `\` as `/`, without empty and `.` segments. None for
/// an absolute path, whose meaning a fix would change.
fn project_path_fix(text: &str) -> Option<String> {
    if text.starts_with(['/', '\\']) {
        return None;
    }
    let segments: Vec<&str> = text
        .split(['/', '\\'])
        .filter(|segment| !matches!(*segment, "" | "."))
        .collect();
    Some(segments.join("/"))
}

/// Whether `text` is one or more words of `[a-z0-9]+` joined by single hyphens: a
/// `capability`, and the slug of a branch (SPEC-0001, Naming grammar).
pub(crate) fn is_hyphenated_words(text: &str) -> bool {
    text.split('-').all(|word| {
        !word.is_empty()
            && word
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
}

/// Whether `text` is a name of the policy: `[a-z][a-z0-9-]*`.
pub(crate) fn is_policy_name(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The words of an ASCII `text` in lower case, every run of other characters written as one
/// hyphen, none at either end. None for a text with a character outside ASCII, which a fix
/// would drop and so change its meaning.
pub(crate) fn hyphenated_words_fix(text: &str) -> Option<String> {
    if !text.is_ascii() {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    Some(words.join("-"))
}

/// What follows the first of `prefixes` that starts `text`, ignoring ASCII case, and one
/// optional separator (`-`, `_` or a space).
fn after_prefix<'a>(text: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let rest = prefixes.iter().find_map(|prefix| {
        let head = text.get(..prefix.len())?;
        head.eq_ignore_ascii_case(prefix)
            .then(|| text.get(prefix.len()..))
            .flatten()
    })?;
    Some(rest.strip_prefix(['-', '_', ' ']).unwrap_or(rest))
}

/// The digits of `text` without leading zeros (`0` for zeros only), when `text` is one or
/// more ASCII digits.
fn number(text: &str) -> Option<&str> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let trimmed = text.trim_start_matches('0');
    Some(if trimmed.is_empty() { "0" } else { trimmed })
}

/// One or more ASCII digits.
pub(crate) fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// `[1-9][0-9]*`.
pub(crate) fn is_counter(text: &str) -> bool {
    all_digits(text) && !text.starts_with('0')
}

/// `[0-9]{4}|[1-9][0-9]{4,}`.
pub(crate) fn is_sequence(text: &str) -> bool {
    all_digits(text) && text.len() >= 4 && (text.len() == 4 || !text.starts_with('0'))
}

/// The value of the ASCII digits of `bytes`; `None` when one is not a digit or the value
/// does not fit.
fn digits(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0_u32, |value, &b| {
        if b.is_ascii_digit() {
            value.checked_mul(10)?.checked_add(u32::from(b - b'0'))
        } else {
            None
        }
    })
}

/// `YYYY-MM-DD`, a day of the Gregorian calendar.
fn is_date(bytes: &[u8]) -> bool {
    let [y0, y1, y2, y3, b'-', m0, m1, b'-', d0, d1] = bytes else {
        return false;
    };
    let (Some(year), Some(month), Some(day)) = (
        digits(&[*y0, *y1, *y2, *y3]),
        digits(&[*m0, *m1]),
        digits(&[*d0, *d1]),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=last).contains(&day)
}

/// `YYYY-MM-DDThh:mm:ss(.f{1,9})?Z`.
fn is_utc_date_time(bytes: &[u8]) -> bool {
    let Some((date, rest)) = bytes.split_at_checked(10) else {
        return false;
    };
    let [b'T', h0, h1, b':', n0, n1, b':', s0, s1, tail @ ..] = rest else {
        return false;
    };
    let (Some(hour), Some(minute), Some(second)) = (
        digits(&[*h0, *h1]),
        digits(&[*n0, *n1]),
        digits(&[*s0, *s1]),
    ) else {
        return false;
    };
    let fraction_ok = match tail {
        [b'Z'] => true,
        [b'.', fraction @ .., b'Z'] => {
            (1..=9).contains(&fraction.len()) && fraction.iter().all(u8::is_ascii_digit)
        }
        _ => false,
    };
    is_date(date) && hour < 24 && minute < 60 && second < 60 && fraction_ok
}

/// A BCP 47 tag limited to language, script, region and variants. Each subtag has one kind by
/// its shape, so reading them in order is enough: 2 or 3 letters (language, first only),
/// 4 letters (script, right after the language), 2 letters or 3 digits (region, after the
/// language or the script), 5 to 8 letters or digits or a digit and 3 (variant, last).
fn is_language_tag(text: &str) -> bool {
    let mut subtags = text.split('-');
    let language = subtags.next().unwrap_or_default();
    if !((2..=3).contains(&language.len()) && language.bytes().all(|b| b.is_ascii_alphabetic())) {
        return false;
    }
    // 1: after the language; 2: after the script; 3: after the region; 4: in the variants.
    let mut stage = 1;
    for subtag in subtags {
        let alpha = subtag.bytes().all(|b| b.is_ascii_alphabetic());
        let alnum = subtag.bytes().all(|b| b.is_ascii_alphanumeric());
        let starts_with_digit = subtag.bytes().next().is_some_and(|b| b.is_ascii_digit());
        stage = if stage == 1 && subtag.len() == 4 && alpha {
            2
        } else if stage <= 2
            && ((subtag.len() == 2 && alpha) || (subtag.len() == 3 && all_digits(subtag)))
        {
            3
        } else if alnum
            && ((5..=8).contains(&subtag.len()) || (subtag.len() == 4 && starts_with_digit))
        {
            4
        } else {
            return false;
        };
    }
    true
}

/// `@` then 1 to 39 letters, digits or single hyphens, not at either end.
fn is_tracker_handle(text: &str) -> bool {
    let Some(name) = text.strip_prefix('@') else {
        return false;
    };
    (1..=39).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

#[cfg(test)]
mod tests {
    use super::{Shape, accepts, part_prefix, shape};
    use wayloop_schemas::Format;

    #[test]
    fn every_identifier_format_checks_its_own_prefix() {
        for format in Format::ALL {
            let name = format.id();
            if let Shape::Sequence(prefixes) = shape(format) {
                let lower: Vec<String> = prefixes.iter().map(|p| p.to_ascii_lowercase()).collect();
                assert_eq!(name, format!("{}-id", lower.join("-or-")));
                for prefix in prefixes {
                    assert!(accepts(format, &format!("{prefix}-0001")), "{name}");
                }
            }
            assert_eq!(
                part_prefix(format).is_some(),
                matches!(shape(format), Shape::Part(_)),
                "{name}"
            );
            if let Shape::Part(prefix) = shape(format) {
                assert_eq!(part_prefix(format), Some(prefix));
                assert!(name.ends_with("-id"), "{name}");
                assert!(accepts(format, &format!("{prefix}1")), "{name}");
            }
        }
    }
}
