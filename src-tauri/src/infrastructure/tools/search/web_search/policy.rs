//! Domain-policy engine for the web-search tool.
//!
//! The `Policy` value is a pair of allow/deny lists of [`DomainPattern`]. All
//! evaluation — "does this host match?", "does this explicit site argument
//! pass?", "what Google `site:` OR-expansion should I append to the query?",
//! "which results survive post-hoc filtering?" — is a pure function of that
//! value plus the input. Nothing here touches the database; a caller (the
//! tool layer) reads `StoredWorkspaceSearchPolicy` from the repository and
//! turns it into a `Policy` via [`Policy::from_stored`].

use super::types::WebSearchResult;

/// A single domain pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainPattern {
    /// The `*` literal — matches any host. Only meaningful in an allowlist
    /// (its presence in a denylist effectively bans all web search).
    Any,
    /// Exact host match, e.g. `github.com` matches `github.com` only.
    Exact(String),
    /// Subdomain match; `.example.com` matches both `example.com` and any
    /// host ending in `.example.com`.
    SubdomainOf(String),
}

impl DomainPattern {
    /// Parses a raw string into a pattern. Trims whitespace and lowercases
    /// the host. Returns `None` for malformed inputs so the caller can drop
    /// garbage without aborting the rest of the policy.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim().to_ascii_lowercase();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed == "*" {
            return Some(Self::Any);
        }
        if let Some(base) = trimmed.strip_prefix('.') {
            if base.is_empty() {
                return None;
            }
            return Some(Self::SubdomainOf(base.to_string()));
        }
        Some(Self::Exact(trimmed))
    }

    /// Returns `true` when the pattern matches `host` (case-insensitive).
    #[must_use]
    pub fn matches(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        match self {
            Self::Any => true,
            Self::Exact(d) => host == *d,
            Self::SubdomainOf(base) => host == *base || host.ends_with(&format!(".{base}")),
        }
    }
}

/// The full domain policy for a workspace.
#[derive(Debug, Clone, Default)]
pub struct Policy {
    allowlist: Vec<DomainPattern>,
    denylist: Vec<DomainPattern>,
}

impl Policy {
    /// The no-op policy: allows every host, blocks none.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds a policy from raw string lists.
    #[must_use]
    pub fn from_raw(allowlist: &[String], denylist: &[String]) -> Self {
        Self {
            allowlist: allowlist
                .iter()
                .filter_map(|s| DomainPattern::parse(s))
                .collect(),
            denylist: denylist
                .iter()
                .filter_map(|s| DomainPattern::parse(s))
                .collect(),
        }
    }

    /// Builds a policy from the DB row shape. A `None` stored value means
    /// "no row recorded" → fall back to the empty policy.
    #[must_use]
    pub fn from_stored(stored: Option<&crate::core::config::StoredWorkspaceSearchPolicy>) -> Self {
        match stored {
            Some(p) => Self::from_raw(&p.allowlist, &p.denylist),
            None => Self::empty(),
        }
    }

    /// Whether the policy has no filters at all. Useful for fast-paths that
    /// skip rewriting / filtering when nothing would change.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.allowlist.is_empty() && self.denylist.is_empty()
    }

    /// Whether `host` is allowed by this policy.
    ///
    /// Semantics (matches spec §4.9):
    /// - A match in the denylist **always** blocks.
    /// - With an empty allowlist, any non-denied host passes.
    /// - With a non-empty allowlist, the host must match at least one entry.
    #[must_use]
    pub fn permits_host(&self, host: &str) -> bool {
        if self.denylist.iter().any(|p| p.matches(host)) {
            return false;
        }
        self.allowlist.is_empty() || self.allowlist.iter().any(|p| p.matches(host))
    }

    /// Runs `permits_host` under a label closer to the LLM vocabulary.
    #[must_use]
    pub fn permits_site(&self, site: &str) -> bool {
        self.permits_host(site)
    }

    /// When no `site` argument was supplied and the allowlist is non-empty,
    /// returns a query suffix of `site:a OR site:b OR site:c` (wrapped in
    /// parentheses if more than one) that the tool appends to the outgoing
    /// query. Returns `None` when no rewrite is needed — either the
    /// allowlist is empty or it contains `*`.
    #[must_use]
    pub fn site_rewrite_query_suffix(&self) -> Option<String> {
        if self.allowlist.is_empty() {
            return None;
        }
        if self
            .allowlist
            .iter()
            .any(|p| matches!(p, DomainPattern::Any))
        {
            return None;
        }
        let parts: Vec<String> = self
            .allowlist
            .iter()
            .filter_map(|p| match p {
                DomainPattern::Any => None,
                DomainPattern::Exact(d) | DomainPattern::SubdomainOf(d) => {
                    Some(format!("site:{d}"))
                }
            })
            .collect();
        match parts.len() {
            0 => None,
            1 => Some(parts.into_iter().next().unwrap_or_default()),
            _ => Some(format!("({})", parts.join(" OR "))),
        }
    }

    /// Filters `results` through the policy. Returns `(kept, any_dropped)`
    /// so the tool can set `filtered_by_policy` on the output only when at
    /// least one result was actually removed.
    #[must_use]
    pub fn filter_results(&self, results: Vec<WebSearchResult>) -> (Vec<WebSearchResult>, bool) {
        if self.is_noop() {
            return (results, false);
        }
        let total = results.len();
        let kept: Vec<WebSearchResult> = results
            .into_iter()
            .filter(|r| self.permits_host(&r.source_domain))
            .collect();
        let dropped = kept.len() < total;
        (kept, dropped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_for(domain: &str) -> WebSearchResult {
        WebSearchResult {
            title: "t".to_string(),
            url: format!("https://{domain}/"),
            snippet: "s".to_string(),
            source_domain: domain.to_string(),
        }
    }

    #[test]
    fn parse_handles_exact_subdomain_and_star() {
        assert_eq!(
            DomainPattern::parse("github.com"),
            Some(DomainPattern::Exact("github.com".to_string()))
        );
        assert_eq!(
            DomainPattern::parse(".rust-lang.org"),
            Some(DomainPattern::SubdomainOf("rust-lang.org".to_string()))
        );
        assert_eq!(DomainPattern::parse("*"), Some(DomainPattern::Any));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(DomainPattern::parse(""), None);
        assert_eq!(DomainPattern::parse("   "), None);
        assert_eq!(DomainPattern::parse("."), None);
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(
            DomainPattern::parse("GitHub.COM"),
            Some(DomainPattern::Exact("github.com".to_string()))
        );
    }

    #[test]
    fn subdomain_pattern_matches_base_and_subdomains() {
        let p = DomainPattern::parse(".example.com").unwrap();
        assert!(p.matches("example.com"));
        assert!(p.matches("blog.example.com"));
        assert!(p.matches("a.b.example.com"));
        assert!(!p.matches("evil-example.com"));
        assert!(!p.matches("example.org"));
    }

    #[test]
    fn exact_pattern_does_not_match_subdomains() {
        let p = DomainPattern::parse("github.com").unwrap();
        assert!(p.matches("github.com"));
        assert!(!p.matches("api.github.com"));
    }

    #[test]
    fn empty_policy_allows_everything() {
        let p = Policy::empty();
        assert!(p.permits_host("anywhere.example"));
        assert!(p.permits_site("github.com"));
        assert!(p.is_noop());
    }

    #[test]
    fn allowlist_restricts_to_listed_hosts() {
        let p = Policy::from_raw(&["github.com".to_string()], &[]);
        assert!(p.permits_host("github.com"));
        assert!(!p.permits_host("pinterest.com"));
    }

    #[test]
    fn denylist_blocks_listed_hosts() {
        let p = Policy::from_raw(&[], &["pinterest.com".to_string()]);
        assert!(!p.permits_host("pinterest.com"));
        assert!(p.permits_host("github.com"));
    }

    #[test]
    fn denylist_wins_when_same_entry_is_in_both_lists() {
        let p = Policy::from_raw(&["example.com".to_string()], &["example.com".to_string()]);
        assert!(!p.permits_host("example.com"));
    }

    #[test]
    fn wildcard_star_in_allowlist_permits_everything() {
        let p = Policy::from_raw(&["*".to_string()], &[]);
        assert!(p.permits_host("anywhere.example"));
    }

    #[test]
    fn subdomain_allowlist_accepts_bare_host() {
        let p = Policy::from_raw(&[".example.com".to_string()], &[]);
        assert!(p.permits_host("example.com"));
        assert!(p.permits_host("blog.example.com"));
        assert!(!p.permits_host("pinterest.com"));
    }

    #[test]
    fn site_rewrite_returns_none_when_allowlist_empty() {
        let p = Policy::from_raw(&[], &[]);
        assert_eq!(p.site_rewrite_query_suffix(), None);
    }

    #[test]
    fn site_rewrite_returns_none_when_allowlist_contains_star() {
        let p = Policy::from_raw(&["*".to_string()], &[]);
        assert_eq!(p.site_rewrite_query_suffix(), None);
    }

    #[test]
    fn site_rewrite_with_single_allowlist_entry_omits_parentheses() {
        let p = Policy::from_raw(&["github.com".to_string()], &[]);
        assert_eq!(
            p.site_rewrite_query_suffix().as_deref(),
            Some("site:github.com")
        );
    }

    #[test]
    fn site_rewrite_expands_multiple_entries_with_or() {
        let p = Policy::from_raw(
            &["github.com".to_string(), ".rust-lang.org".to_string()],
            &[],
        );
        assert_eq!(
            p.site_rewrite_query_suffix().as_deref(),
            Some("(site:github.com OR site:rust-lang.org)")
        );
    }

    #[test]
    fn filter_results_drops_denylisted_urls() {
        let p = Policy::from_raw(&[], &["pinterest.com".to_string()]);
        let (kept, dropped) =
            p.filter_results(vec![result_for("github.com"), result_for("pinterest.com")]);
        assert!(dropped);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].source_domain, "github.com");
    }

    #[test]
    fn filter_results_with_empty_allowlist_keeps_everything() {
        let p = Policy::empty();
        let (kept, dropped) = p.filter_results(vec![result_for("a.com"), result_for("b.com")]);
        assert!(!dropped);
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn filter_results_restricts_to_allowlist() {
        let p = Policy::from_raw(&[".rust-lang.org".to_string()], &[]);
        let (kept, dropped) = p.filter_results(vec![
            result_for("blog.rust-lang.org"),
            result_for("pinterest.com"),
        ]);
        assert!(dropped);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn filter_results_returns_filtered_true_even_when_all_were_dropped() {
        let p = Policy::from_raw(&[], &[".bad.example".to_string()]);
        let (kept, dropped) = p.filter_results(vec![result_for("sub.bad.example")]);
        assert!(dropped);
        assert!(kept.is_empty());
    }

    #[test]
    fn from_stored_handles_none_as_empty_policy() {
        let p = Policy::from_stored(None);
        assert!(p.is_noop());
    }

    #[test]
    fn from_stored_reads_lists() {
        let stored = crate::core::config::StoredWorkspaceSearchPolicy {
            allowlist: vec!["github.com".to_string()],
            denylist: vec!["pinterest.com".to_string()],
        };
        let p = Policy::from_stored(Some(&stored));
        assert!(p.permits_host("github.com"));
        assert!(!p.permits_host("pinterest.com"));
    }
}
