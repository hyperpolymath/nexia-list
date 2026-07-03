// SPDX-License-Identifier: MPL-2.0
//! Wiki-link parsing: extract `[[Title]]` / `[[Title|alias]]` references.
//!
//! Pure and total — never panics on any input (fuzzed). Shared by the
//! markdown exchange layer now and the editor autocomplete later.

/// A parsed `[[Target]]` or `[[Target|alias]]` reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// The referenced note title (trimmed, never empty).
    pub target: String,
    /// Optional display alias after a `|`.
    pub alias: Option<String>,
    /// Byte range of the whole `[[...]]` token in the source.
    pub start: usize,
    pub end: usize,
}

/// Extract every well-formed wiki-link, left to right, non-overlapping.
/// Unclosed `[[` and empty `[[]]` are ignored.
pub fn parse_wikilinks(content: &str) -> Vec<WikiLink> {
    let bytes = content.as_bytes();
    let mut links = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let open = i;
            let inner_start = i + 2;
            // Find the closing "]]" at or after inner_start.
            if let Some(close) = find_close(bytes, inner_start) {
                let inner = &content[inner_start..close];
                if let Some(link) = parse_inner(inner, open, close + 2) {
                    links.push(link);
                }
                i = close + 2;
                continue;
            } else {
                // No closing bracket anywhere after: nothing more to find.
                break;
            }
        }
        i += 1;
    }
    links
}

/// The distinct titles referenced, de-duplicated, order preserved.
pub fn wikilink_targets(content: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for link in parse_wikilinks(content) {
        if !seen.iter().any(|t| t == &link.target) {
            seen.push(link.target);
        }
    }
    seen
}

fn find_close(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b']' {
            return Some(i);
        }
        // A new "[[" before any "]]" means the first "[[" is dangling; let the
        // outer loop restart from here by reporting no close.
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            return None;
        }
        i += 1;
    }
    None
}

fn parse_inner(inner: &str, start: usize, end: usize) -> Option<WikiLink> {
    let (target_raw, alias) = match inner.split_once('|') {
        Some((t, a)) => (t, Some(a.trim().to_string()).filter(|s| !s.is_empty())),
        None => (inner, None),
    };
    let target = target_raw.trim();
    if target.is_empty() {
        return None;
    }
    Some(WikiLink {
        target: target.to_string(),
        alias,
        start,
        end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_link() {
        let links = parse_wikilinks("see [[Alpha]] here");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Alpha");
        assert_eq!(links[0].alias, None);
        assert_eq!(
            &"see [[Alpha]] here"[links[0].start..links[0].end],
            "[[Alpha]]"
        );
    }

    #[test]
    fn aliased_and_trimmed() {
        let links = parse_wikilinks("[[  Beta Note | shown ]]");
        assert_eq!(links[0].target, "Beta Note");
        assert_eq!(links[0].alias, Some("shown".to_string()));
    }

    #[test]
    fn multiple_and_dedup() {
        let content = "[[A]] and [[B]] and [[A]] again";
        assert_eq!(parse_wikilinks(content).len(), 3);
        assert_eq!(wikilink_targets(content), vec!["A", "B"]);
    }

    #[test]
    fn ignores_empty_and_unclosed() {
        assert!(parse_wikilinks("[[]] [[  ]] [[unclosed").is_empty());
    }

    #[test]
    fn unicode_offsets_are_valid() {
        let content = "héllo [[Naïve]] wörld";
        let links = parse_wikilinks(content);
        assert_eq!(links[0].target, "Naïve");
        // Slicing at the reported offsets must not panic on a char boundary.
        assert_eq!(&content[links[0].start..links[0].end], "[[Naïve]]");
    }
}
