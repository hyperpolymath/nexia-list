// SPDX-License-Identifier: MPL-2.0
//! `nexia-export`: print the AsciiDoc view of a notebook file, or check that
//! a committed view is current. See docs/design/notebooks-in-repositories.adoc.

use nexia_core::exchange::to_asciidoc;
use nexia_core::Notebook;
use std::process::ExitCode;

const USAGE: &str =
    "usage: nexia-export <notebook.nexia.json> [--group-by KEY] [--check VIEW.adoc]";

fn run() -> Result<bool, String> {
    let mut input = None;
    let mut group_by = None;
    let mut check = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--group-by" => group_by = Some(args.next().ok_or(USAGE)?),
            "--check" => check = Some(args.next().ok_or(USAGE)?),
            _ if arg.starts_with('-') || input.is_some() => return Err(USAGE.to_string()),
            _ => input = Some(arg),
        }
    }
    let input = input.ok_or(USAGE)?;
    let json = std::fs::read_to_string(&input).map_err(|e| format!("{input}: {e}"))?;
    let (notebook, report) = Notebook::load_json(&json).map_err(|e| format!("{input}: {e}"))?;
    if !report.is_clean() {
        // The view describes the repaired notebook, so the file and the view
        // would disagree. Refuse; the notebook must be saved clean first.
        return Err(format!(
            "{input}: notebook needs repair on load: {report:?}"
        ));
    }
    let view = to_asciidoc(&notebook, group_by.as_deref());
    match check {
        None => {
            print!("{view}");
            Ok(true)
        }
        Some(path) => {
            let committed = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            if committed != view {
                eprintln!("{path}: out of date with {input}; regenerate it");
            }
            Ok(committed == view)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
