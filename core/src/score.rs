//! Ranking helpers shared by the matcher, the decoder and the engine.

use std::cmp::Ordering;

/// Descending by score; equal scores keep their order (sorts are stable).
pub fn descending(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

/// Sort by a score computed once per item, highest first, keeping the order of ties, like
/// Python's `sorted(..., key=lambda x: -score(x))`.
pub fn sort_by_score<T>(items: Vec<T>, score: impl Fn(&T) -> f64) -> Vec<T> {
    let mut scored: Vec<(f64, T)> = items.into_iter().map(|item| (score(&item), item)).collect();
    scored.sort_by(|a, b| descending(a.0, b.0));
    scored.into_iter().map(|(_, item)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::sort_by_score;

    #[test]
    fn ties_keep_their_order() {
        let sorted = sort_by_score(
            vec![("a", 1.0), ("b", 2.0), ("c", 1.0), ("d", -0.0), ("e", 0.0)],
            |x| x.1,
        );
        assert_eq!(sorted.iter().map(|x| x.0).collect::<String>(), "bacde");
    }
}
