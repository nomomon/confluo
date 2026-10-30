//! Provider anchors for "Copy link to comment".

use gv_engine::ProviderKind;

/// Link to one comment: GitHub `#issuecomment-{id}` / `#discussion_r{id}` (database id decoded from the
/// node id), GitLab `#note_{id}`, Azure `?discussionId={thread}`; else the PR.
pub fn comment_link(web_url: &str, kind: ProviderKind, id: &str) -> String {
    match kind {
        ProviderKind::GitLab if id.chars().all(|c| c.is_ascii_digit()) => {
            format!("{web_url}#note_{id}")
        }
        ProviderKind::GitHub => match (id.split_once('_'), github_database_id(id)) {
            (Some(("IC", _)), Some(n)) => format!("{web_url}#issuecomment-{n}"),
            (Some(("PRRC", _)), Some(n)) => format!("{web_url}#discussion_r{n}"),
            (Some(("PRR", _)), Some(n)) => format!("{web_url}#pullrequestreview-{n}"),
            _ => web_url.to_string(),
        },
        ProviderKind::Azure => match id.split_once('/') {
            Some((t, _)) => format!("{web_url}?discussionId={t}"),
            None => web_url.to_string(),
        },
        _ => web_url.to_string(),
    }
}

/// Database id inside a GitHub node id (`IC_kwDO...`: base64url of a msgpack `[0, repo id, id]`).
pub fn github_database_id(node: &str) -> Option<u64> {
    let (_, b64) = node.split_once('_')?;
    let bytes = b64url(b64)?;
    let mut it = bytes.into_iter();
    if it.next()? & 0xf0 != 0x90 {
        return None;
    }
    let mut last = None;
    while let Some(tag) = it.next() {
        let n = match tag {
            0x00..=0x7f => tag as u64,
            0xcc => it.next()? as u64,
            0xcd => (0..2).try_fold(0u64, |a, _| Some(a << 8 | it.next()? as u64))?,
            0xce => (0..4).try_fold(0u64, |a, _| Some(a << 8 | it.next()? as u64))?,
            0xcf => (0..8).try_fold(0u64, |a, _| Some(a << 8 | it.next()? as u64))?,
            _ => return None,
        };
        last = Some(n);
    }
    last
}

fn b64url(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        _ => None,
    };
    let (mut out, mut acc, mut bits) = (vec![], 0u32, 0);
    for c in s.bytes().filter(|c| *c != b'=') {
        acc = acc << 6 | val(c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_links_per_provider() {
        let gh = "https://github.com/a/b/pull/4";
        assert_eq!(
            comment_link(gh, ProviderKind::GitHub, "IC_kwDOVCkx488AAAABhJw4UA"),
            format!("{gh}#issuecomment-6519797840")
        );
        assert_eq!(comment_link(gh, ProviderKind::GitHub, "nonsense"), gh);
        assert_eq!(
            comment_link(
                "https://gl/x/-/merge_requests/1",
                ProviderKind::GitLab,
                "42"
            ),
            "https://gl/x/-/merge_requests/1#note_42"
        );
        assert_eq!(
            comment_link(
                "https://dev.azure.com/o/p/_git/r/pullrequest/5",
                ProviderKind::Azure,
                "12/3"
            ),
            "https://dev.azure.com/o/p/_git/r/pullrequest/5?discussionId=12"
        );
    }
}
