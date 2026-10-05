// SPDX-License-Identifier: MPL-2.0
//! Notebook performance budget at scale: 10,000 notes, 50,000 links.
//!
//! Run: `cargo bench -p nexia-core --bench notebook_perf`
//! (or `bun run bench:rust`; add `-- --smoke` for a quick build-and-run check). Release profile, std-only harness: no extra
//! dependencies, deterministic input (fixed-seed generator), and a non-zero
//! exit when an interactive query exceeds its budget — so CI can gate on it.
//!
//! Budgets (p95, native release build):
//! * substring search over title + content ........ < 10 ms
//! * backlink lookup .............................. < 10 ms
//!
//! Everything else is reported but not gated: it is not on the per-keystroke
//! path. Numbers are for the native build; the WASM build is typically a small
//! constant factor slower and is not measured here.

use nexia_core::{NoteId, Notebook};
use std::hint::black_box;
use std::time::{Duration, Instant};

const NOTES: usize = 10_000;
const LINKS: usize = 50_000;
const BUDGET: Duration = Duration::from_millis(10);

/// Small deterministic PRNG (SplitMix64) so every run measures the same graph.
struct Rng(u64);

impl Rng {
    /// Next pseudo-random 64-bit value.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform index in `0..n`.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const WORDS: &[&str] = &[
    "graph",
    "note",
    "spatial",
    "canvas",
    "agent",
    "prototype",
    "link",
    "idea",
    "draft",
    "review",
    "garden",
    "atlas",
    "memory",
    "context",
    "outline",
    "theme",
    "signal",
    "pattern",
    "lambda",
    "delta",
    "field",
    "query",
    "archive",
    "sketch",
    "thread",
    "summary",
    "source",
];

/// Build the benchmark notebook: titled, ~40-word notes on a grid, `LINKS`
/// distinct directed links, and a handful of attributes and computed fields.
fn build(rng: &mut Rng) -> (Notebook, Vec<NoteId>) {
    let mut nb = Notebook::new("bench");
    let mut ids = Vec::with_capacity(NOTES);
    for i in 0..NOTES {
        let id = nb.create_note(format!("Note {i} {}", WORDS[i % WORDS.len()]));
        let content: Vec<&str> = (0..40).map(|_| WORDS[rng.below(WORDS.len())]).collect();
        let note = nb.get_note_mut(&id).expect("just created");
        note.content = content.join(" ");
        note.position = Some(nexia_core::Point2D::new(
            (i % 100) as f64 * 240.0,
            (i / 100) as f64 * 180.0,
        ));
        if i % 10 == 0 {
            note.set_attribute("status", serde_json::json!("todo"));
            note.set_computed("words", "(count (words (content self)))");
        }
        ids.push(id);
    }
    let mut made = 0;
    while made < LINKS {
        let (from, to) = (ids[rng.below(NOTES)], ids[rng.below(NOTES)]);
        if from != to && !nb.get_note(&from).expect("exists").links_to(&to) {
            nb.link_notes(from, to).expect("both exist");
            made += 1;
        }
    }
    (nb, ids)
}

/// Timing summary over a set of samples.
struct Stats {
    p50: Duration,
    p95: Duration,
    max: Duration,
}

/// Run `op` `iters` times and summarise the per-call wall time.
fn measure(iters: usize, mut op: impl FnMut(usize)) -> Stats {
    let mut samples: Vec<Duration> = (0..iters)
        .map(|i| {
            let start = Instant::now();
            op(i);
            start.elapsed()
        })
        .collect();
    samples.sort();
    let at = |q: f64| samples[((samples.len() - 1) as f64 * q).round() as usize];
    Stats {
        p50: at(0.50),
        p95: at(0.95),
        max: at(1.0),
    }
}

/// Format a duration as fractional milliseconds.
fn ms(d: Duration) -> String {
    format!("{:>9.3}", d.as_secs_f64() * 1e3)
}

/// Build the benchmark notebook, time each operation, print the table, and
/// exit non-zero if a gated budget is exceeded.
fn main() {
    // `-- --smoke`: a few iterations only, to check the bench builds and runs.
    let smoke = std::env::args().any(|a| a == "--smoke");
    let iters = if smoke { 5 } else { 200 };

    let mut rng = Rng(0x00C0_FFEE);
    let built = Instant::now();
    let (mut nb, ids) = build(&mut rng);
    let build_time = built.elapsed();
    let backlink_total: usize = ids.iter().map(|id| nb.get_backlinks(id).len()).sum();
    assert_eq!(nb.len(), NOTES);
    assert_eq!(
        backlink_total, LINKS,
        "backlink index must mirror every link"
    );

    println!("nexia-core notebook_perf — {NOTES} notes, {LINKS} links, {iters} iterations");
    println!("(built in {} ms)\n", ms(build_time).trim());
    println!(
        "{:<34} {:>9} {:>9} {:>9}  budget",
        "operation", "p50 ms", "p95 ms", "max ms"
    );

    let mut failures = Vec::new();
    let mut row = |name: &str, s: Stats, gated: bool| {
        let verdict = if !gated {
            "-".to_string()
        } else if s.p95 <= BUDGET {
            "PASS <10ms".to_string()
        } else {
            failures.push(name.to_string());
            "FAIL >10ms".to_string()
        };
        println!(
            "{name:<34} {} {} {}  {verdict}",
            ms(s.p50),
            ms(s.p95),
            ms(s.max)
        );
    };

    let queries = [
        "garden",
        "Note 42",
        "lambda delta",
        "zzz-no-match",
        "NOTE 9999",
    ];
    row(
        "search (substring, title+content)",
        measure(iters, |i| {
            black_box(nb.search(queries[i % queries.len()]));
        }),
        true,
    );
    row(
        "backlinks (single note)",
        measure(iters, |i| {
            black_box(nb.get_backlinks(&ids[(i * 7919) % NOTES]));
        }),
        true,
    );
    row(
        "agent query (run_query)",
        measure(iters, |i| {
            black_box(nb.run_query(queries[i % queries.len()]));
        }),
        false,
    );
    let edit_iters = iters.min(50);
    row(
        "edit content w/ [[wikilink]]",
        measure(edit_iters, |i| {
            let id = ids[(i * 104_729) % NOTES];
            black_box(nb.set_content(&id, format!("edited {i} see [[Note {i} graph]]")));
        }),
        false,
    );
    let io_iters = if smoke { 1 } else { 5 };
    let mut json = String::new();
    row(
        "save (serialize whole notebook)",
        measure(io_iters, |_| {
            json = serde_json::to_string(&nb).expect("serializes");
        }),
        false,
    );
    row(
        "load (parse + migrate + repair)",
        measure(io_iters, |_| {
            black_box(Notebook::load_json(&json).expect("own output loads"));
        }),
        false,
    );
    println!(
        "\nserialized size: {:.1} MiB",
        json.len() as f64 / (1024.0 * 1024.0)
    );

    if failures.is_empty() {
        println!("\nall gated budgets met");
    } else {
        eprintln!("\nBUDGET EXCEEDED: {}", failures.join(", "));
        std::process::exit(1);
    }
}
