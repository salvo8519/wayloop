//! The default policy (SPEC-0021 R1, SPEC-0001 R21): the policy that applies when the input of a
//! command gives none, equal to the policy of this repository.
//!
//! The types and the scopes are those of `scripts/conventions.py`, and the areas those of the
//! `area:` labels of `roadmap/roadmap.yaml`. The crate embeds no file outside itself, so these
//! lists are a deliberate copy: `tests/process/test_conventions.py` keeps them equal to their
//! sources. A check that reads the policy of an input without one reports the default in
//! `meta.defaults_applied` with [`DEFAULT_POLICY`] at `/policy`, through [`applied`].

use wayloop_schemas::validate::Policy;
use wayloop_schemas::value::PolicyName;
use wayloop_schemas::{DefaultApplied, DefaultReason, Error, Pointer};

/// The reason of the default policy in `meta.defaults_applied`.
pub(crate) const DEFAULT_POLICY: DefaultReason = DefaultReason::new("default_policy");

/// The member of the input that holds the policy.
pub(crate) const POLICY: &str = "policy";

/// Types of commits, branches and pull request titles.
pub(crate) const TYPES: [&str; 10] = [
    "feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "revert",
];

/// Scopes of commits and pull request titles.
pub(crate) const SCOPES: [&str; 26] = [
    "core",
    "runtime",
    "ledger",
    "state",
    "tracker",
    "survey",
    "schemas",
    "security",
    "cli",
    "python",
    "typescript",
    "mcp",
    "plugins",
    "docs",
    "i18n",
    "brand",
    "specs",
    "adr",
    "agents",
    "project",
    "roadmap",
    "ci",
    "deps",
    "release",
    "scripts",
    "benchmark",
];

/// Areas of epics and tasks, without the `area:` prefix.
pub(crate) const AREAS: [&str; 14] = [
    "core",
    "runtime",
    "cli",
    "docs",
    "agents",
    "infra",
    "tracker",
    "survey",
    "sdk",
    "mcp",
    "plugins",
    "release",
    "security",
    "benchmark",
];

/// The default policy.
pub(crate) fn default_policy() -> Policy {
    let names = |list: &[&str]| list.iter().map(|n| PolicyName::new(*n)).collect();
    Policy {
        types: names(&TYPES),
        scopes: names(&SCOPES),
        areas: names(&AREAS),
    }
}

/// The entry of `meta.defaults_applied` that reports the default policy, or a failure of the
/// engine when it cannot be written.
pub(crate) fn applied() -> Result<DefaultApplied, Box<Error>> {
    let value = serde_json::to_value(default_policy()).map_err(|e| {
        Box::new(Error::internal(format!(
            "The default policy cannot be written: {e}."
        )))
    })?;
    let pointer = Pointer::root().join(POLICY);
    let default = DefaultApplied::new(pointer, value, DEFAULT_POLICY)
        .ok_or_else(|| Box::new(Error::internal("The default policy is written as null.")))?;
    Ok(default.with_message(
        "The input gives no policy: the default policy applies, the policy of the Wayloop \
         repository.",
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use wayloop_schemas::validate::Policy;

    use super::{AREAS, SCOPES, TYPES, applied, default_policy};
    use crate::schema::format::is_policy_name;

    #[test]
    fn the_lists_are_within_the_bounds_and_the_form_of_a_policy() {
        assert!(TYPES.len() <= Policy::TYPES_MAX);
        assert!(SCOPES.len() <= Policy::SCOPES_MAX);
        assert!(AREAS.len() <= Policy::AREAS_MAX);
        for name in TYPES.iter().chain(&SCOPES).chain(&AREAS) {
            assert!(is_policy_name(name), "{name}");
        }
    }

    #[test]
    fn the_lists_have_no_repeated_name() {
        for list in [&TYPES[..], &SCOPES[..], &AREAS[..]] {
            let mut sorted = list.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), list.len(), "{list:?}");
        }
    }

    #[test]
    fn the_default_is_reported_at_policy_with_its_value() {
        let default = applied().unwrap();
        assert_eq!(default.pointer().as_str(), "/policy");
        assert_eq!(default.reason().id(), "default_policy");
        assert_eq!(default.document(), None);
        assert_eq!(default.value()["types"], json!(TYPES));
        assert_eq!(default.value()["scopes"], json!(SCOPES));
        assert_eq!(default.value()["areas"], json!(AREAS));
        assert_eq!(default_policy().types.len(), TYPES.len());
    }
}
