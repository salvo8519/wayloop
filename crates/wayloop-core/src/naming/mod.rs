//! The naming rules of SPEC-0001 (R7 to R11; section Naming grammar): pure functions on titles,
//! branch names, commit headers and pull request links, each returning the errors of its
//! grammar with the codes of [`crate::artifacts::codes`]. The checks across the set that call
//! them live in [`crate::artifacts`].

pub(crate) mod titles;
