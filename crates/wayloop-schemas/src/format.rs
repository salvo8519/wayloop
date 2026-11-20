//! The named formats of the contracts (ADR 0030 G2).
//!
//! One enum lists every named format a contract may use. The generator writes a format only
//! through it, and the format checker of `wayloop-core` matches on it exhaustively: a format
//! added here does not compile until the core can check it, and a format the core lacks cannot
//! be written in a schema. The accepted forms are documented with the interpreter
//! (`wayloop_core::schema`).

/// A named format of the subset of JSON Schema that the core interprets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Format {
    /// `date`: an RFC 3339 `full-date`.
    Date,
    /// `utc-date-time`: an RFC 3339 date-time in UTC, written with `Z`.
    UtcDateTime,
    /// `language-tag`: a BCP 47 language tag of language, script, region and variants.
    LanguageTag,
    /// `tracker-handle`: `@` and a user name of the tracker.
    TrackerHandle,
    /// `brief-id`: `BRIEF-` and a sequence.
    BriefId,
    /// `council-id`: `COUNCIL-` and a sequence.
    CouncilId,
    /// `gate-id`: `GATE-` and a sequence.
    GateId,
    /// `epic-id`: `EPIC-` and a sequence.
    EpicId,
    /// `story-id`: `STORY-` and a sequence.
    StoryId,
    /// `spec-id`: `SPEC-` and a sequence.
    SpecId,
    /// `change-id`: `CHANGE-` and a sequence.
    ChangeId,
    /// `task-id`: `TASK-` and a sequence.
    TaskId,
    /// `milestone-id`: `M` and a number from 0.
    MilestoneId,
    /// `handover-id`: `HANDOVER-<issue>-<k>`, both numbers from 1.
    HandoverId,
    /// `requirement-id`: `R` and a number from 1.
    RequirementId,
    /// `criterion-id`: `AC` and a number from 1.
    CriterionId,
    /// `decision-id`: `D` and a number from 1.
    DecisionId,
    /// `question-id`: `Q` and a number from 1.
    QuestionId,
}

impl Format {
    /// Every named format, in the order of the table of the interpreter.
    pub const ALL: [Format; 18] = [
        Format::Date,
        Format::UtcDateTime,
        Format::LanguageTag,
        Format::TrackerHandle,
        Format::BriefId,
        Format::CouncilId,
        Format::GateId,
        Format::EpicId,
        Format::StoryId,
        Format::SpecId,
        Format::ChangeId,
        Format::TaskId,
        Format::MilestoneId,
        Format::HandoverId,
        Format::RequirementId,
        Format::CriterionId,
        Format::DecisionId,
        Format::QuestionId,
    ];

    /// The name of the format, as written in the `format` keyword of a schema.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Format::Date => "date",
            Format::UtcDateTime => "utc-date-time",
            Format::LanguageTag => "language-tag",
            Format::TrackerHandle => "tracker-handle",
            Format::BriefId => "brief-id",
            Format::CouncilId => "council-id",
            Format::GateId => "gate-id",
            Format::EpicId => "epic-id",
            Format::StoryId => "story-id",
            Format::SpecId => "spec-id",
            Format::ChangeId => "change-id",
            Format::TaskId => "task-id",
            Format::MilestoneId => "milestone-id",
            Format::HandoverId => "handover-id",
            Format::RequirementId => "requirement-id",
            Format::CriterionId => "criterion-id",
            Format::DecisionId => "decision-id",
            Format::QuestionId => "question-id",
        }
    }

    /// The format called `name`; `None` when no named format has that name.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| format.name() == name)
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
            assert_eq!(Format::named(format.name()), Some(format));
        }
        let names: Vec<&str> = Format::ALL.iter().map(|f| f.name()).collect();
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
            assert_eq!(Format::named(unknown), None, "{unknown}");
        }
    }
}
