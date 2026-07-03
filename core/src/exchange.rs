// SPDX-License-Identifier: MPL-2.0
//! Import / export between a `Notebook` and portable formats: a Markdown
//! vault (one file per note, Obsidian-style `[[wiki-links]]`) and OPML.
//!
//! The Markdown round-trip preserves titles and link topology. Titles are the
//! link currency, so a clean round-trip assumes distinct titles; duplicates
//! resolve to the first match (documented limitation, never panics).

use crate::note::{Note, NoteId};
use crate::notebook::Notebook;
use crate::wikilink::wikilink_targets;
use std::collections::HashMap;

/// One exported Markdown file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownFile {
    pub name: String,
    pub content: String,
}

/// Export every note as a Markdown file: front-matter, body, and a trailing
/// `## Links` section of `[[Title]]` references for outgoing links.
pub fn to_markdown(notebook: &Notebook) -> Vec<MarkdownFile> {
    let titles: HashMap<NoteId, String> = notebook
        .all_notes()
        .map(|n| (n.id, n.title.clone()))
        .collect();

    let mut used_names: HashMap<String, u32> = HashMap::new();
    let mut files = Vec::new();

    for note in notebook.all_notes() {
        let mut body = String::new();
        body.push_str("---\n");
        body.push_str(&format!("id: {}\n", note.id));
        body.push_str(&format!("created_at: {}\n", note.created_at.to_rfc3339()));
        body.push_str(&format!("modified_at: {}\n", note.modified_at.to_rfc3339()));
        if !note.attributes.is_empty() {
            if let Ok(attrs) = serde_json::to_string(&note.attributes) {
                body.push_str(&format!("attributes: {attrs}\n"));
            }
        }
        body.push_str("---\n\n");

        let title = if note.title.is_empty() {
            "Untitled"
        } else {
            &note.title
        };
        body.push_str(&format!("# {title}\n\n"));

        if !note.content.is_empty() {
            body.push_str(&note.content);
            if !note.content.ends_with('\n') {
                body.push('\n');
            }
        }

        if !note.links.is_empty() {
            body.push_str("\n## Links\n\n");
            for target in &note.links {
                let target_title = titles.get(target).map(String::as_str).unwrap_or("Unknown");
                body.push_str(&format!("- [[{target_title}]]\n"));
            }
        }

        files.push(MarkdownFile {
            name: unique_filename(title, &mut used_names),
            content: body,
        });
    }

    files
}

/// Export the notebook as an OPML 2.0 outline.
pub fn to_opml(notebook: &Notebook) -> String {
    let titles: HashMap<NoteId, String> = notebook
        .all_notes()
        .map(|n| (n.id, n.title.clone()))
        .collect();

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<opml version=\"2.0\">\n");
    out.push_str(&format!(
        "  <head><title>{}</title></head>\n",
        xml_escape(&notebook.name)
    ));
    out.push_str("  <body>\n");
    for note in notebook.all_notes() {
        let text = xml_escape(if note.title.is_empty() {
            "Untitled"
        } else {
            &note.title
        });
        if note.links.is_empty() {
            out.push_str(&format!("    <outline text=\"{text}\"/>\n"));
        } else {
            out.push_str(&format!("    <outline text=\"{text}\">\n"));
            for target in &note.links {
                let child = titles.get(target).map(String::as_str).unwrap_or("Unknown");
                out.push_str(&format!(
                    "      <outline text=\"{}\"/>\n",
                    xml_escape(child)
                ));
            }
            out.push_str("    </outline>\n");
        }
    }
    out.push_str("  </body>\n");
    out.push_str("</opml>\n");
    out
}

/// Build a fresh notebook from a set of Markdown files. Title comes from the
/// first `# H1` (falling back to the filename stem); `[[Title]]` references
/// become links, and any that name a not-yet-seen title create a placeholder
/// note (Obsidian semantics).
pub fn from_markdown_vault(files: &[MarkdownFile]) -> Notebook {
    let mut notebook = Notebook::new("Imported");

    // Pass 1: create a note per file, remembering title -> id.
    let mut by_title: HashMap<String, NoteId> = HashMap::new();
    let mut parsed: Vec<(NoteId, String)> = Vec::new(); // (id, body-with-wikilinks)

    for file in files {
        let (_front, rest) = strip_front_matter(&file.content);
        let (title, body_after_title) = extract_title(rest, &file.name);
        let (body, _links_section) = split_links_section(body_after_title);

        let mut note = Note::new(title.clone());
        note.content = body.trim().to_string();
        let id = note.id;
        notebook.add_note(note);
        by_title.entry(title.to_lowercase()).or_insert(id);
        // Collect link targets from the whole file (content + links section).
        parsed.push((id, rest.to_string()));
    }

    // Pass 2: resolve wiki-links; create placeholders for unresolved titles.
    for (id, source) in parsed {
        for target in wikilink_targets(&source) {
            let key = target.to_lowercase();
            let target_id = match by_title.get(&key) {
                Some(existing) => *existing,
                None => {
                    let placeholder = Note::new(target.clone());
                    let pid = placeholder.id;
                    notebook.add_note(placeholder);
                    by_title.insert(key, pid);
                    pid
                }
            };
            let _ = notebook.link_notes(id, target_id);
        }
    }

    notebook.rebuild_backlinks();
    notebook
}

fn unique_filename(title: &str, used: &mut HashMap<String, u32>) -> String {
    let mut stem: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    stem = stem.trim().to_string();
    if stem.is_empty() {
        stem = "note".to_string();
    }
    let count = used.entry(stem.clone()).or_insert(0);
    let name = if *count == 0 {
        format!("{stem}.md")
    } else {
        format!("{stem}-{count}.md")
    };
    *count += 1;
    name
}

/// Return (front_matter, rest) splitting a leading `---\n...\n---\n` block.
fn strip_front_matter(content: &str) -> (&str, &str) {
    let trimmed = content.trim_start_matches('\u{feff}');
    if let Some(after_open) = trimmed.strip_prefix("---\n") {
        if let Some(end) = after_open.find("\n---") {
            let front = &after_open[..end];
            let rest = &after_open[end + 4..];
            return (front, rest.trim_start_matches('\n'));
        }
    }
    ("", content)
}

/// Extract a leading `# Title` line; fall back to the filename stem.
fn extract_title<'a>(body: &'a str, filename: &str) -> (String, &'a str) {
    let body = body.trim_start();
    if let Some(rest) = body.strip_prefix("# ") {
        if let Some(nl) = rest.find('\n') {
            return (rest[..nl].trim().to_string(), &rest[nl + 1..]);
        }
        return (rest.trim().to_string(), "");
    }
    let stem = filename.strip_suffix(".md").unwrap_or(filename).to_string();
    (stem, body)
}

/// Split off a trailing `## Links` section so it does not pollute content.
fn split_links_section(body: &str) -> (&str, &str) {
    if let Some(idx) = body.find("\n## Links") {
        (&body[..idx], &body[idx..])
    } else if let Some(stripped) = body.strip_prefix("## Links") {
        ("", stripped)
    } else {
        (body, "")
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linked_notebook() -> (Notebook, NoteId, NoteId) {
        let mut nb = Notebook::new("Test");
        let a = nb.create_note("Alpha");
        let b = nb.create_note("Beta");
        if let Some(note) = nb.get_note_mut(&a) {
            note.content = "Alpha body".into();
        }
        nb.link_notes(a, b).unwrap();
        (nb, a, b)
    }

    #[test]
    fn markdown_has_front_matter_and_links() {
        let (nb, _a, _b) = linked_notebook();
        let files = to_markdown(&nb);
        assert_eq!(files.len(), 2);
        let alpha = files.iter().find(|f| f.name == "Alpha.md").unwrap();
        assert!(alpha.content.contains("# Alpha"));
        assert!(alpha.content.contains("## Links"));
        assert!(alpha.content.contains("[[Beta]]"));
    }

    #[test]
    fn markdown_roundtrip_preserves_titles_and_links() {
        let (nb, _a, _b) = linked_notebook();
        let files = to_markdown(&nb);
        let imported = from_markdown_vault(&files);

        assert_eq!(imported.len(), 2);
        let titles: Vec<_> = {
            let mut t: Vec<_> = imported.all_notes().map(|n| n.title.clone()).collect();
            t.sort();
            t
        };
        assert_eq!(titles, vec!["Alpha", "Beta"]);

        let alpha = imported.all_notes().find(|n| n.title == "Alpha").unwrap();
        let beta = imported.all_notes().find(|n| n.title == "Beta").unwrap();
        assert!(alpha.links_to(&beta.id));
        assert_eq!(alpha.content, "Alpha body");
        assert_eq!(imported.get_backlinks(&beta.id), vec![alpha.id]);
    }

    #[test]
    fn import_creates_placeholder_for_unresolved_link() {
        let files = vec![MarkdownFile {
            name: "One.md".into(),
            content: "# One\n\nlink to [[Ghost]]\n".into(),
        }];
        let nb = from_markdown_vault(&files);
        assert_eq!(nb.len(), 2);
        assert!(nb.all_notes().any(|n| n.title == "Ghost"));
    }

    #[test]
    fn opml_is_well_formed_and_escaped() {
        let mut nb = Notebook::new("My <Notes> & Co");
        nb.create_note("A & B");
        let opml = to_opml(&nb);
        assert!(opml.contains("<opml version=\"2.0\">"));
        assert!(opml.contains("My &lt;Notes&gt; &amp; Co"));
        assert!(opml.contains("text=\"A &amp; B\""));
    }
}
