//! Masked (secret) single-line inputs: the text is drawn as one bullet per byte and only
//! printable ASCII is accepted, so a content offset maps to `offset * BULLET_LEN` on screen.

use super::model::{Row, Rows};

const BULLET: char = '\u{2022}';
pub const BULLET_LEN: usize = BULLET.len_utf8();

/// What is drawn for `len` bytes of secret text.
pub fn masked(len: usize) -> String {
    std::iter::repeat_n(BULLET, len).collect()
}

/// Keep only what a token can contain (printable ASCII, no spaces or newlines).
pub fn accept(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_graphic()).collect()
}

/// Rows laid out on the bullets, converted back to content offsets.
pub fn unscale(rows: Vec<Row>) -> Rows {
    Rows(
        rows.into_iter()
            .map(|r| Row {
                start: r.start / BULLET_LEN,
                end: r.end / BULLET_LEN,
                soft: r.soft,
                stops: r
                    .stops
                    .into_iter()
                    .map(|(o, x)| (o / BULLET_LEN, x))
                    .collect(),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_and_maps_offsets() {
        assert_eq!(masked(3).len(), 3 * BULLET_LEN);
        assert_eq!(accept(" ab c\n\u{e9}d "), "abcd");
        let rows = vec![Row {
            start: 0,
            end: 2 * BULLET_LEN,
            soft: false,
            stops: vec![(0, 0.), (BULLET_LEN, 5.), (2 * BULLET_LEN, 10.)],
        }];
        let r = &unscale(rows).0[0];
        assert_eq!((r.start, r.end), (0, 2));
        assert_eq!(r.stops, vec![(0, 0.), (1, 5.), (2, 10.)]);
    }
}
