//! The named formats of the contracts (ADR 0030 G2).
//!
//! One enum lists every named format a contract may use. The generator writes a format only
//! through it, and the format checker of `wayloop-core` matches on it exhaustively: a format
//! added here does not compile until the core can check it, and a format the core lacks cannot
//! be written in a schema. The accepted forms are documented with the interpreter
//! (`wayloop_core::schema`).

crate::contract_enum! {
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
        /// `BRIEF-` and a sequence.
        BriefId = "brief-id",
        /// `COUNCIL-` and a sequence.
        CouncilId = "council-id",
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
            Format::BriefId => 4,
            Format::CouncilId => 5,
            Format::GateId => 6,
            Format::EpicId => 7,
            Format::StoryId => 8,
            Format::SpecId => 9,
            Format::ChangeId => 10,
            Format::TaskId => 11,
            Format::MilestoneId => 12,
            Format::HandoverId => 13,
            Format::RequirementId => 14,
            Format::CriterionId => 15,
            Format::DecisionId => 16,
            Format::QuestionId => 17,
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
                "brief-id",
                "council-id",
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
            ]
        );
        for unknown in ["email", "uint32", "", "Date"] {
            assert_eq!(Format::from_id(unknown), None, "{unknown}");
        }
    }
}
