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

#[cfg(test)]
mod tests {
    use super::Format;

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
            Format::BriefId => 6,
            Format::CouncilId => 7,
            Format::BriefOrCouncilId => 8,
            Format::GateId => 9,
            Format::EpicId => 10,
            Format::StoryId => 11,
            Format::SpecId => 12,
            Format::ChangeId => 13,
            Format::TaskId => 14,
            Format::MilestoneId => 15,
            Format::HandoverId => 16,
            Format::RequirementId => 17,
            Format::CriterionId => 18,
            Format::DecisionId => 19,
            Format::QuestionId => 20,
            Format::JsonPointer => 21,
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
}
