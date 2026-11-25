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

/// Each item of `items` whose `key` another item shares, with one other carrier as `related`,
/// in the order of `items`: the duplicates of the set (R8, R14). No error copies a list that
/// grows with the input (decided by the maintainer on 2026-11-25): every later carrier names
/// the first, and the first names the second, so the report stays linear in the carriers.
pub(super) fn shared<'t, T>(
    items: &'t [T],
    key: impl Fn(&'t T) -> &'t str + Copy,
    related: impl Fn(&T) -> Related,
) -> Vec<(&'t T, Related)> {
    shared_in(items, &carriers(items, key), key, related)
}

/// [`shared`] with the map `carriers` of `items` already built by [`carriers`].
pub(super) fn shared_in<'t, T>(
    items: &'t [T],
    carriers: &BTreeMap<&str, Vec<&'t T>>,
    key: impl Fn(&'t T) -> &'t str,
    related: impl Fn(&T) -> Related,
) -> Vec<(&'t T, Related)> {
    items
        .iter()
        .filter_map(|item| {
            let [first, second, ..] = carriers.get(key(item))?.as_slice() else {
                return None;
            };
            let named = if core::ptr::eq(*first, item) {
                second
            } else {
                first
            };
            Some((item, related(named)))
        })
        .collect()
}
