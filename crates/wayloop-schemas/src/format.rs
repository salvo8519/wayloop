//! The named formats of the contracts (ADR 0030 G2).
//!
//! One enum lists every named format a contract may use. The generator writes a format only
//! through it, and the format checker of `wayloop-core` matches on it exhaustively: a format
//! added here does not compile until the core can check it, and a format the core lacks cannot
//! be written in a schema. The accepted forms are documented with the interpreter
//! (`wayloop_core::schema`).

crate::closed_enum! {
    /// A named format of the subset of JSON Schema that the core interprets; its identifier is
    /// the value of the `format` keyword.
    pub enum Format {
        /// An RFC 3339 `full-date`.
        Date = "date",
        /// An RFC 3339 date-time in UTC, written with `Z`.
        UtcDateTime = "utc-date-time",
        /// A BCP 47 language tag of language, script, region and variants.
        LanguageTag = "language-tag",
        /// `@` and a user name of the tracker.
        TrackerHandle = "tracker-handle",
        /// A path relative to the root of the project, in one spelling: segments joined by `/`,
        /// none empty, `.` or `..`; no `\` and no drive letter, so that it never leaves the
        /// project.
        ProjectPath = "project-path",
        /// Lower-case words of ASCII letters and digits joined by single hyphens: the
        /// capability of a living specification, the slug of its folder.
        Capability = "capability",
        /// A name of the project policy, a type of commits, a scope or an area: one lower-case
        /// word of ASCII letters, digits and hyphens that starts with a letter.
        PolicyName = "policy-name",
        /// `BRIEF-` and a sequence.
        BriefId = "brief-id",
        /// `COUNCIL-` and a sequence.
        CouncilId = "council-id",
        /// `BRIEF-` or `COUNCIL-` and a sequence: what a gate issue submits for approval.
        BriefOrCouncilId = "brief-or-council-id",
        /// `GATE-` and a sequence.
        GateId = "gate-id",
        /// `EPIC-` and a sequence.
        EpicId = "epic-id",
        /// `STORY-` and a sequence.
        StoryId = "story-id",
        /// `SPEC-` and a sequence.
        SpecId = "spec-id",
        /// `CHANGE-` and a sequence.
        ChangeId = "change-id",
        /// `TASK-` and a sequence.
        TaskId = "task-id",
        /// `M` and a number from 0.
        MilestoneId = "milestone-id",
        /// `HANDOVER-<issue>-<k>`, both numbers from 1.
        HandoverId = "handover-id",
        /// `R` and a number from 1.
        RequirementId = "requirement-id",
        /// `AC` and a number from 1.
        CriterionId = "criterion-id",
        /// `D` and a number from 1.
        DecisionId = "decision-id",
        /// `Q` and a number from 1.
        QuestionId = "question-id",
        /// An RFC 6901 JSON pointer: empty, or `/`-prefixed reference tokens in which `~` is
        /// followed by `0` or `1`.
        JsonPointer = "json-pointer",
    }
}

/// The most characters of a name of the project policy, a value of the format `policy-name`
/// (#267, decided by the maintainer on 2026-11-25): the schema of the name states it as
/// `maxLength`, so that an error that lists names of the policy in `expected` stays bounded.
pub const POLICY_NAME_MAX_CHARS: usize = 32;

impl Format {
    /// The most characters of a value of the format, which every schema that uses the format
    /// states as `maxLength` beside it; `None` for a format that bounds no length of its own.
    #[must_use]
    pub const fn max_chars(self) -> Option<usize> {
        if matches!(self, Self::PolicyName) {
            Some(POLICY_NAME_MAX_CHARS)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, POLICY_NAME_MAX_CHARS};

    // Exhaustive: a new format does not compile until it gets a position here, and then the
    // test fails until ALL lists it in that position.
    const fn position(format: Format) -> usize {
        match format {
            Format::Date => 0,
            Format::UtcDateTime => 1,
            Format::LanguageTag => 2,
            Format::TrackerHandle => 3,
            Format::ProjectPath => 4,
            Format::Capability => 5,
            Format::PolicyName => 6,
            Format::BriefId => 7,
            Format::CouncilId => 8,
            Format::BriefOrCouncilId => 9,
            Format::GateId => 10,
            Format::EpicId => 11,
            Format::StoryId => 12,
            Format::SpecId => 13,
            Format::ChangeId => 14,
            Format::TaskId => 15,
            Format::MilestoneId => 16,
            Format::HandoverId => 17,
            Format::RequirementId => 18,
            Format::CriterionId => 19,
            Format::DecisionId => 20,
            Format::QuestionId => 21,
            Format::JsonPointer => 22,
        }
    }

    #[test]
    fn all_lists_every_format_once_in_order() {
        for (i, format) in Format::ALL.into_iter().enumerate() {
            assert_eq!(position(format), i);
        }
    }

    #[test]
    fn names_and_formats_correspond_one_to_one() {
        for format in Format::ALL {
            assert_eq!(Format::from_id(format.id()), Some(format));
        }
        let names: Vec<&str> = Format::ALL.iter().map(|f| f.id()).collect();
        assert_eq!(
            names,
            [
                "date",
                "utc-date-time",
                "language-tag",
                "tracker-handle",
                "project-path",
                "capability",
                "policy-name",
                "brief-id",
                "council-id",
                "brief-or-council-id",
                "gate-id",
                "epic-id",
                "story-id",
                "spec-id",
                "change-id",
                "task-id",
                "milestone-id",
                "handover-id",
                "requirement-id",
                "criterion-id",
                "decision-id",
                "question-id",
                "json-pointer",
            ]
        );
        for unknown in ["email", "uint32", "", "Date"] {
            assert_eq!(Format::from_id(unknown), None, "{unknown}");
        }
    }

    #[test]
    fn only_the_policy_name_bounds_its_length() {
        for format in Format::ALL {
            let bound = (format == Format::PolicyName).then_some(POLICY_NAME_MAX_CHARS);
            assert_eq!(format.max_chars(), bound, "{format}");
        }
        assert_eq!(POLICY_NAME_MAX_CHARS, 32);
    }
}
