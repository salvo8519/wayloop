//! Values that more than one document of the set carries: the identifiers (R14) and the titles
//! of epics and tasks (R8). Each check keeps its own code and message; this module finds the
//! carriers and names them.

use std::collections::BTreeMap;

use crate::Related;

/// The items of `items` by their `key`, each list in the order of `items`: the one map of the
/// values that several documents may carry, built once per value checked.
pub(super) fn carriers<'t, T>(
    items: &'t [T],
    key: impl Fn(&'t T) -> &'t str,
) -> BTreeMap<&'t str, Vec<&'t T>> {
    let mut carriers: BTreeMap<&str, Vec<&T>> = BTreeMap::new();
    for item in items {
        carriers.entry(key(item)).or_default().push(item);
    }
    carriers
}

/// Each item of `items` whose `key` another item shares, with the others as `related`, in the
/// order of `items`: the duplicates of the set (R8, R14).
pub(super) fn shared<'t, T>(
    items: &'t [T],
    key: impl Fn(&'t T) -> &'t str + Copy,
    related: impl Fn(&T) -> Related,
) -> Vec<(&'t T, Vec<Related>)> {
    shared_in(items, &carriers(items, key), key, related)
}

/// [`shared`] with the map `carriers` of `items` already built by [`carriers`].
pub(super) fn shared_in<'t, T>(
    items: &'t [T],
    carriers: &BTreeMap<&str, Vec<&'t T>>,
    key: impl Fn(&'t T) -> &'t str,
    related: impl Fn(&T) -> Related,
) -> Vec<(&'t T, Vec<Related>)> {
    items
        .iter()
        .filter_map(|item| {
            let others: Vec<Related> = carriers
                .get(key(item))?
                .iter()
                .filter(|other| !core::ptr::eq(**other, item))
                .map(|other| related(other))
                .collect();
            (!others.is_empty()).then_some((item, others))
        })
        .collect()
}

/// The documents of `others` as a message names them: `document 3` or `documents 3, 5`.
pub(super) fn documents(others: &[Related]) -> String {
    let listed: Vec<String> = others.iter().map(|o| o.document().to_string()).collect();
    let noun = if listed.len() == 1 {
        "document"
    } else {
        "documents"
    };
    format!("{noun} {}", listed.join(", "))
}
