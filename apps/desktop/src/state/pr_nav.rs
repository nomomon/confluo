//! Cmd+Up/Down (adjacent PR) and Option+Cmd+Up/Down (adjacent section) over the visible list:
//! sections in order, collapsed and empty ones skipped, filters already applied by the caller.

use super::Section;

/// Where a step lands: a PR, or past either end of the list.
#[derive(Debug, PartialEq, Eq)]
pub enum Step<T> {
    To(T),
    End,
}

/// Group `(id, section)` rows (already filtered, in list order) into the sections on screen.
pub fn visible_sections<T: Clone>(
    rows: &[(T, Section)],
    collapsed: &[Section],
) -> Vec<(Section, Vec<T>)> {
    Section::ALL
        .into_iter()
        .filter(|s| !collapsed.contains(s))
        .map(|s| {
            (
                s,
                rows.iter()
                    .filter(|(_, rs)| *rs == s)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, ids)| !ids.is_empty())
        .collect()
}

/// The PR `delta` (+1 / -1) away from `current`; no wrap. Unknown or no current starts at an end.
pub fn adjacent<T: Clone + PartialEq>(
    sections: &[(Section, Vec<T>)],
    current: Option<&T>,
    delta: isize,
) -> Step<T> {
    let flat: Vec<&T> = sections.iter().flat_map(|(_, ids)| ids).collect();
    if flat.is_empty() {
        return Step::End;
    }
    let pos = current.and_then(|c| flat.iter().position(|x| *x == c));
    let next = match pos {
        Some(i) => i as isize + delta,
        None if delta > 0 => 0,
        None => flat.len() as isize - 1,
    };
    if next < 0 || next as usize >= flat.len() {
        Step::End
    } else {
        Step::To(flat[next as usize].clone())
    }
}

/// First PR of the section `delta` away from `current`'s section; no wrap.
pub fn adjacent_section<T: Clone + PartialEq>(
    sections: &[(Section, Vec<T>)],
    current: Option<&T>,
    delta: isize,
) -> Step<T> {
    if sections.is_empty() {
        return Step::End;
    }
    let pos = current.and_then(|c| sections.iter().position(|(_, ids)| ids.contains(c)));
    let next = match pos {
        Some(i) => i as isize + delta,
        None if delta > 0 => 0,
        None => sections.len() as isize - 1,
    };
    if next < 0 || next as usize >= sections.len() {
        Step::End
    } else {
        Step::To(sections[next as usize].1[0].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Section::*;

    fn rows() -> Vec<(u32, Section)> {
        vec![
            (1, NeedsYou),
            (2, NeedsYou),
            (3, Drafts),
            (4, Yours),
            (5, Waiting),
        ]
    }

    #[test]
    fn sections_follow_list_order_and_skip_collapsed_and_empty() {
        let s = visible_sections(&rows(), &[Drafts]);
        assert_eq!(
            s,
            vec![(NeedsYou, vec![1, 2]), (Yours, vec![4]), (Waiting, vec![5])]
        );
    }

    #[test]
    fn next_and_previous_cross_sections_without_wrapping() {
        let s = visible_sections(&rows(), &[Drafts]);
        assert_eq!(adjacent(&s, Some(&2), 1), Step::To(4));
        assert_eq!(adjacent(&s, Some(&4), -1), Step::To(2));
        assert_eq!(adjacent(&s, Some(&5), 1), Step::End);
        assert_eq!(adjacent(&s, Some(&1), -1), Step::End);
        // 3 sits in a collapsed section: start from an end.
        assert_eq!(adjacent(&s, Some(&3), 1), Step::To(1));
        assert_eq!(adjacent(&s, None, -1), Step::To(5));
    }

    #[test]
    fn filters_are_the_callers_rows() {
        // A search that leaves only 1 and 5.
        let filtered: Vec<_> = rows()
            .into_iter()
            .filter(|(id, _)| [1, 5].contains(id))
            .collect();
        let s = visible_sections(&filtered, &[]);
        assert_eq!(adjacent(&s, Some(&1), 1), Step::To(5));
        assert_eq!(adjacent::<u32>(&[], None, 1), Step::End);
    }

    #[test]
    fn section_jumps_land_on_first_pr() {
        let s = visible_sections(&rows(), &[]);
        // Section order: Needs you [1, 2], Yours [4], Drafts [3], Waiting [5].
        assert_eq!(adjacent_section(&s, Some(&2), 1), Step::To(4));
        assert_eq!(adjacent_section(&s, Some(&4), 1), Step::To(3));
        assert_eq!(adjacent_section(&s, Some(&4), -1), Step::To(1));
        assert_eq!(adjacent_section(&s, Some(&5), 1), Step::End);
        assert_eq!(adjacent_section(&s, Some(&1), -1), Step::End);
        let c = visible_sections(&rows(), &[Drafts]);
        assert_eq!(adjacent_section(&c, Some(&4), 1), Step::To(5));
    }
}
