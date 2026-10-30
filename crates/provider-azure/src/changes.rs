//! The last iteration's change list, fetched once per head and shared by the file count and the diff.

use gv_core::*;

use crate::diff::{entries, Entry};
use crate::{lock, AzureProvider, API};

/// Whether `url` is on `host` itself (https), so the token may be sent with it.
pub(crate) fn own_host(host: &str, url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https" && u.host_str().is_some_and(|h| h.eq_ignore_ascii_case(host))
    })
}

impl AzureProvider {
    /// Blob entries of the last iteration against the target (two requests, then memoized per head).
    pub(crate) async fn change_entries(&self, id: &ChangeRequestId) -> ProviderResult<Vec<Entry>> {
        let key = id.key();
        let head = lock(&self.heads).get(&key).cloned().unwrap_or_default();
        if let Some((h, e)) = lock(&self.changes).get(&key) {
            if !head.is_empty() && *h == head {
                return Ok(e.clone());
            }
        }
        let base = self.pr_url(id)?;
        let its = self.get(&format!("{base}/iterations?{API}")).await?;
        let Some(last) = its["value"]
            .as_array()
            .and_then(|a| a.iter().filter_map(|i| i["id"].as_u64()).max())
        else {
            return Ok(vec![]);
        };
        let mut all = vec![];
        let mut skip = 0u64;
        for _ in 0..10 {
            let v = self
                .get(&format!(
                    "{base}/iterations/{last}/changes?$top=1000&$skip={skip}&$compareTo=0&{API}"
                ))
                .await?;
            all.extend(entries(&v));
            match v["nextSkip"].as_u64() {
                Some(n) if n > skip => skip = n,
                _ => break,
            }
        }
        lock(&self.changes).insert(key, (head, all.clone()));
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::own_host;

    #[test]
    fn token_stays_on_its_host() {
        assert!(own_host(
            "dev.azure.com",
            "https://dev.azure.com/o/_api/_common/identityImage?id=1"
        ));
        assert!(!own_host("dev.azure.com", "http://dev.azure.com/x"));
        assert!(!own_host(
            "dev.azure.com",
            "https://evil.example/dev.azure.com"
        ));
        assert!(!own_host(
            "dev.azure.com",
            "https://dev.azure.com.evil.example/x"
        ));
    }
}
