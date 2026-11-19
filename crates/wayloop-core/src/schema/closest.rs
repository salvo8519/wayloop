//! The closest declared key to a misspelt one (ADR 0009 C3, `fix`).
//!
//! The distance is the optimal string alignment distance, the restricted form of the
//! Damerau-Levenshtein distance: insertions, deletions, substitutions and transpositions of
//! two adjacent characters each cost one, and no substring is edited twice. It is computed on
//! Unicode scalar values, with three rows of the dynamic programme, so memory grows with the
//! length of the declared key only.

/// Keys longer than this, in characters, get no fix: the cost of the distance grows with the
/// product of the two lengths, and the key comes from the input.
pub(crate) const MAX_KEY_CHARS: usize = 64;

/// The optimal string alignment distance between `a` and `b`, in edits.
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // Rows i - 2, i - 1 and i of the table, each of b.len() + 1 cells.
    let mut before: Vec<usize> = vec![0; b.len().saturating_add(1)];
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current: Vec<usize> = vec![0; b.len().saturating_add(1)];
    let mut last_a: Option<char> = None;
    for (i, &ca) in a.iter().enumerate() {
        let row = i.saturating_add(1);
        if let Some(first) = current.first_mut() {
            *first = row;
        }
        let mut last_b: Option<char> = None;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let deletion = cell(&previous, j.saturating_add(1)).saturating_add(1);
            let insertion = cell(&current, j).saturating_add(1);
            let substitution = cell(&previous, j).saturating_add(cost);
            let mut best = deletion.min(insertion).min(substitution);
            if last_a == Some(cb) && last_b == Some(ca) {
                // A transposition: `last_b` is set, so `j` is at least 1.
                best = best.min(cell(&before, j.saturating_sub(1)).saturating_add(1));
            }
            if let Some(slot) = current.get_mut(j.saturating_add(1)) {
                *slot = best;
            }
            last_b = Some(cb);
        }
        last_a = Some(ca);
        core::mem::swap(&mut before, &mut previous);
        core::mem::swap(&mut previous, &mut current);
    }
    cell(&previous, b.len())
}

/// The cell `index` of a row; every index used is within the row by construction.
fn cell(row: &[usize], index: usize) -> usize {
    row.get(index).copied().unwrap_or(usize::MAX)
}

/// The candidate closest to `key`, if one is at a distance of at most a third of the length
/// of `key`; ties go to the first candidate in the order given.
pub(crate) fn closest<'a>(
    key: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let length = key.chars().count();
    if length > MAX_KEY_CHARS {
        return None;
    }
    let mut best: Option<(usize, &str)> = None;
    for candidate in candidates {
        // The difference of the lengths bounds the distance from below: skip what cannot win.
        if candidate.chars().count().abs_diff(length).saturating_mul(3) > length {
            continue;
        }
        let d = distance(key, candidate);
        if d.saturating_mul(3) <= length && best.is_none_or(|(b, _)| d < b) {
            best = Some((d, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

#[cfg(test)]
mod tests {
    use super::{MAX_KEY_CHARS, closest, distance};
    use proptest::prelude::*;

    #[test]
    fn known_distances() {
        assert_eq!(distance("", ""), 0);
        assert_eq!(distance("abc", ""), 3);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("problema", "problem"), 1);
        assert_eq!(distance("titolo", "title"), 2);
        assert_eq!(distance("xy", "yx"), 1);
        assert_eq!(distance("statsu", "status"), 1);
        // Optimal string alignment, not the unrestricted distance (which gives 2).
        assert_eq!(distance("ca", "abc"), 3);
        assert_eq!(distance("\u{e8}t\u{e0}", "et\u{e0}"), 1);
    }

    #[test]
    fn the_closest_key_is_within_a_third_of_the_length_of_the_key() {
        let keys = ["problem", "title", "status", "goals"];
        assert_eq!(closest("problema", keys), Some("problem"));
        assert_eq!(closest("titolo", keys), Some("title"));
        assert_eq!(closest("titl", keys), Some("title"));
        // Two edits on five characters is more than a third.
        assert_eq!(closest("tutle", ["title"]), Some("title"));
        assert_eq!(closest("tutlz", ["title"]), None);
        assert_eq!(closest("zzzz", keys), None);
        assert_eq!(closest("", keys), None);
        assert_eq!(closest("ab", ["a"]), None);
        // A difference of lengths of exactly a third is still a candidate.
        assert_eq!(closest("abc", ["abcd"]), Some("abcd"));
    }

    #[test]
    fn ties_go_to_the_first_candidate() {
        assert_eq!(closest("abcd", ["abce", "abcf"]), Some("abce"));
        assert_eq!(closest("abcd", ["abcf", "abce"]), Some("abcf"));
        assert_eq!(closest("abcd", ["abcf", "abcd"]), Some("abcd"));
    }

    #[test]
    fn a_long_key_gets_no_fix() {
        let long = "a".repeat(MAX_KEY_CHARS + 1);
        let near = "a".repeat(MAX_KEY_CHARS);
        assert_eq!(closest(&long, [near.as_str()]), None);
        let limit = "a".repeat(MAX_KEY_CHARS);
        assert_eq!(closest(&limit, [near.as_str()]), Some(near.as_str()));
    }

    /// The textbook definition, with the whole table, as an independent reference.
    fn reference(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut d = vec![vec![0_usize; b.len() + 1]; a.len() + 1];
        for (i, row) in d.iter_mut().enumerate() {
            row[0] = i;
        }
        for (j, cell) in d[0].iter_mut().enumerate() {
            *cell = j;
        }
        for i in 1..=a.len() {
            for j in 1..=b.len() {
                let cost = usize::from(a[i - 1] != b[j - 1]);
                d[i][j] = (d[i - 1][j] + 1)
                    .min(d[i][j - 1] + 1)
                    .min(d[i - 1][j - 1] + cost);
                if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                    d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
                }
            }
        }
        d[a.len()][b.len()]
    }

    proptest! {
        #[test]
        fn agrees_with_the_full_table(a in "[abc\u{e8}]{0,9}", b in "[abc\u{e8}]{0,9}") {
            prop_assert_eq!(distance(&a, &b), reference(&a, &b));
            prop_assert_eq!(distance(&a, &b), distance(&b, &a));
            prop_assert_eq!(distance(&a, &a), 0);
        }
    }
}
