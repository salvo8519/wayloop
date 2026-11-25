//! The naming rules of SPEC-0001 (R7 to R11; section Naming grammar): pure functions on titles,
//! branch names, commit headers and pull request links, each returning the errors of its
//! grammar with the codes of [`crate::artifacts::codes`]. The checks across the set that call
//! them live in [`crate::artifacts`].

pub(crate) mod branch;
pub(crate) mod commit;
pub(crate) mod titles;

use std::collections::BTreeSet;

use wayloop_schemas::value::PolicyName;

/// A list of names of the policy, such as its types or its scopes, indexed once for the rules
/// that look names up in it: the cost of a lookup does not grow with the documents checked.
#[derive(Debug, Clone)]
pub(crate) struct Names<'p> {
    /// The list as the policy gives it, for `expected`.
    list: &'p [PolicyName],
    index: BTreeSet<&'p str>,
}

impl<'p> Names<'p> {
    /// The names of `list`.
    pub(crate) fn new(list: &'p [PolicyName]) -> Self {
        let index = list.iter().map(PolicyName::as_str).collect();
        Self { list, index }
    }

    /// Whether `name` is one of the names, spelt as the policy spells it.
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.index.contains(name)
    }

    /// The names in the order of the policy, the values of `expected`.
    pub(crate) fn expected(&self) -> impl Iterator<Item = &'p str> {
        self.list.iter().map(PolicyName::as_str)
    }
}
