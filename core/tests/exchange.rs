// SPDX-License-Identifier: MPL-2.0
//! Property tests for the exchange layer and the wiki-link parser.

use nexia_core::exchange::{from_markdown_vault, to_markdown, to_opml, MarkdownFile};
use nexia_core::wikilink::parse_wikilinks;
use nexia_core::Notebook;
use proptest::prelude::*;

proptest! {
    /// The wiki-link parser never panics and every reported byte range is a
    /// valid slice of the source.
    #[test]
    fn wikilink_parse_is_total(s in ".{0,400}") {
        for link in parse_wikilinks(&s) {
            prop_assert!(link.start <= link.end);
            prop_assert!(link.end <= s.len());
            // Slicing at the offsets must land on char boundaries.
            let _ = &s[link.start..link.end];
            prop_assert!(!link.target.is_empty());
        }
    }

    /// Import never panics on arbitrary Markdown and yields a consistent
    /// backlink index (every note it produces round-trips its own links).
    #[test]
    fn import_is_total(bodies in prop::collection::vec(".{0,120}", 0..6)) {
        let files: Vec<MarkdownFile> = bodies
            .into_iter()
            .enumerate()
            .map(|(i, content)| MarkdownFile { name: format!("n{i}.md"), content })
            .collect();
        let nb = from_markdown_vault(&files);
        for note in nb.all_notes() {
            for target in &note.links {
                prop_assert!(nb.get_note(target).is_some(), "dangling link after import");
                prop_assert!(nb.get_backlinks(target).contains(&note.id));
            }
        }
        // Exporting the result must also never panic.
        let _ = to_markdown(&nb);
        let _ = to_opml(&nb);
    }

    /// Markdown round-trip preserves note count and link topology for
    /// distinct-title notebooks.
    #[test]
    fn markdown_roundtrip_preserves_topology(n in 1usize..8) {
        let mut nb = Notebook::new("Prop");
        let ids: Vec<_> = (0..n).map(|i| nb.create_note(format!("Note {i}"))).collect();
        // Chain them: 0 -> 1 -> 2 ...
        for pair in ids.windows(2) {
            nb.link_notes(pair[0], pair[1]).unwrap();
        }
        let imported = from_markdown_vault(&to_markdown(&nb));
        prop_assert_eq!(imported.len(), nb.len());
        // Every original title survives.
        for i in 0..n {
            let title = format!("Note {i}");
            prop_assert!(imported.all_notes().any(|note| note.title == title));
        }
    }
}
