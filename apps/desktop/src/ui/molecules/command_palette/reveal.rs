//! Keyboard reveal: which scroller child to bring into view for a row.

/// Child index (headers and rows are siblings) to reveal for each row: the group header for a
/// group's first row, so moving up shows its heading too; otherwise the row itself.
pub fn targets(groups: &[&str]) -> Vec<usize> {
    let mut out = Vec::with_capacity(groups.len());
    let (mut last, mut child) = ("", 0);
    for g in groups {
        out.push(child);
        if *g != last {
            last = g;
            child += 2;
        } else {
            child += 1;
        }
    }
    out
}

/// Wrapping move over `n` rows.
pub fn step(sel: usize, d: isize, n: usize) -> usize {
    if n == 0 {
        0
    } else {
        (sel as isize + d).rem_euclid(n as isize) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_row_of_group_reveals_its_header() {
        // children: [H a][r0][r1][H b][r2]
        assert_eq!(targets(&["a", "a", "b"]), vec![0, 2, 3]);
        assert_eq!(targets(&[]), Vec::<usize>::new());
    }

    #[test]
    fn step_wraps() {
        assert_eq!(step(0, -1, 3), 2);
        assert_eq!(step(2, 1, 3), 0);
        assert_eq!(step(0, 1, 0), 0);
    }
}
