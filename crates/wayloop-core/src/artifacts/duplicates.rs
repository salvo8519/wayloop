//! Values that more than one document of the set carries: the identifiers (R14) and the titles
//! of epics and tasks (R8). Each check keeps its own code and message; this module finds the
//! carriers and names them.

use std::collections::BTreeMap;

use crate::Related;

/// Each item of `items` whose `key` another item shares, with the others as `related`, in the
/// order of `items`: the duplicates of the set (R8, R14).
pub(super) fn shared<T>(
    items: &[T],
    key: impl Fn(&T) -> &str,
    related: impl Fn(&T) -> Related,
) -> Vec<(&T, Vec<Related>)> {
    let mut carriers: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (position, item) in items.iter().enumerate() {
        carriers.entry(key(item)).or_default().push(position);
    }
    items
        .iter()
        .enumerate()
        .filter_map(|(position, item)| {
            let others: Vec<Related> = carriers
                .get(key(item))?
                .iter()
                .filter(|other| **other != position)
                .filter_map(|other| items.get(*other))
                .map(&related)
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
