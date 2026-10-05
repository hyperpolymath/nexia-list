// SPDX-License-Identifier: MPL-2.0
//
// nexia_host.js — host bindings for Store.affine.
//
// Host carve-out (with affinescript-tea's tea_host.js, the only JavaScript in
// the UI): wraps the wasm-bindgen `WasmNotebook` (core/src/wasm.rs) and the
// browser storage/file APIs, and implements the `nx_*` externs declared in
// ui/src/Store.affine as globals. It keeps a *read model* — note records in
// the shape Store.affine's `Note` struct expects — updated incrementally from
// the views and deltas the core returns, so rendering a 10k-note notebook
// never re-marshals it. It makes no UI decisions.

import init, { WasmNotebook } from "./wasm/nexia_core.js";

const AUTOSAVE_KEY = "nexia.autosave";
const AUTOSAVE_DELAY_MS = 300;
/** Longest a pending change waits for a write while edits keep coming. */
const AUTOSAVE_MAX_WAIT_MS = 1000;
const IDB_NAME = "nexia";
const IDB_STORE = "kv";

/** The live notebook (set by nx_boot). */
let nb = null;

// ── Read model ─────────────────────────────────────────────────────────────

/** id -> UI note record. */
const notes = new Map();
/** Notes sorted by title, or null when it must be rebuilt. */
let sorted = null;
/** id -> index into `sorted` (valid while `sorted` is). */
const sortedIndex = new Map();
/** Revision: bumps on every successful mutation or load. */
let rev = 0;
/** id -> evaluated computed fields, valid for the current revision. */
const fieldCache = new Map();

/** Read-model version counter (see Store.affine `Note.ver`). */
let nextVer = 0;

/** Convert a core NoteView into Store.affine's `Note` record. */
function toNote(v) {
  return {
    id: v.id,
    title: v.title,
    content: v.content,
    placed: v.position != null,
    x: v.position?.x ?? 0,
    y: v.position?.y ?? 0,
    w: v.size?.[0] ?? 0,
    h: v.size?.[1] ?? 0,
    links: v.links ?? [],
    created: v.created_at,
    modified: v.modified_at,
    field_count: Object.keys(v.computed ?? {}).length,
    ver: ++nextVer,
  };
}

/** Title order, then id, so the order is total and stable. */
function byTitle(a, b) {
  const byName = a.title.localeCompare(b.title);
  if (byName !== 0) return byName;
  if (a.id === b.id) return 0;
  return a.id < b.id ? -1 : 1;
}

/** Advance the read-model revision and invalidate evaluated computed fields. */
function touched() {
  rev += 1;
  fieldCache.clear();
}

/** Upsert one note view; keeps the sorted array when the title is unchanged. */
function upsert(view) {
  const note = toNote(view);
  const prev = notes.get(note.id);
  notes.set(note.id, note);
  if (sorted && prev && prev.title === note.title) {
    sorted[sortedIndex.get(note.id)] = note;
  } else {
    sorted = null;
  }
  return note;
}

/** Drop a note from the read model. */
function drop(id) {
  if (notes.delete(id)) sorted = null;
}

/** Rebuild the whole read model from a core snapshot. */
function reloadAll() {
  notes.clear();
  sorted = null;
  const snap = nb.snapshot();
  for (const view of Object.values(snap.notes))
    notes.set(view.id, toNote(view));
  touched();
}

/** Notes sorted by title (rebuilt lazily after creates, deletes, renames). */
function sortedNotes() {
  if (!sorted) {
    sorted = Array.from(notes.values()).sort(byTitle);
    sortedIndex.clear();
    sorted.forEach((n, i) => {
      sortedIndex.set(n.id, i);
    });
  }
  return sorted;
}

/** An empty, unplaced note (returned for an unknown id). */
const NO_NOTE = Object.freeze({
  id: "",
  title: "",
  content: "",
  placed: false,
  x: 0,
  y: 0,
  w: 0,
  h: 0,
  links: [],
  created: "",
  modified: "",
  field_count: 0,
  ver: 0,
});

// ── Outcomes ───────────────────────────────────────────────────────────────

/** A successful outcome. */
const ok = (id = "", notice = "") => ({ ok: true, error: "", notice, id });
/** A failed outcome carrying the core's (or browser's) message. */
const fail = (e) => ({
  ok: false,
  error: e instanceof Error ? e.message : String(e),
  notice: "",
  id: "",
});

/**
 * Run a core mutation returning one note view, update the read model and
 * schedule autosave. Return an outcome identifying the note; caught errors
 * become failed outcomes without rolling back changes already made.
 */
function noteOp(fn) {
  try {
    const note = upsert(fn());
    touched();
    schedule();
    return ok(note.id);
  } catch (e) {
    return fail(e);
  }
}

/**
 * Run a core mutation returning a delta `{ changed, removed }`, update the
 * read model and schedule autosave. Return an outcome carrying the supplied
 * id; caught errors become failed outcomes without rolling back prior changes.
 */
function deltaOp(fn, id = "") {
  try {
    const delta = fn();
    for (const view of delta.changed ?? []) upsert(view);
    for (const gone of delta.removed ?? []) drop(gone);
    touched();
    schedule();
    return ok(id);
  } catch (e) {
    return fail(e);
  }
}

// ── IndexedDB ──────────────────────────────────────────────────────────────

/** Open the key-value store, creating it on upgrade; reject on an open error. */
function idbOpen() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(IDB_STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

/**
 * Run op against the key-value store with the given transaction mode. Resolve
 * with the request result only after the transaction commits, then close the
 * connection. Reject on open, transaction or callback errors, including aborts.
 */
async function idbRequest(mode, op) {
  const db = await idbOpen();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(IDB_STORE, mode);
      const req = op(tx.objectStore(IDB_STORE));
      let result;
      req.onsuccess = () => {
        result = req.result;
      };
      tx.oncomplete = () => resolve(result);
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
  } finally {
    db.close();
  }
}

/** Read a stored value, or undefined when absent; propagate IndexedDB failures. */
const idbGet = (key) => idbRequest("readonly", (s) => s.get(key));
/** Store or replace a value; resolve with its key on commit or reject on failure. */
const idbSet = (key, value) =>
  idbRequest("readwrite", (s) => s.put(value, key));
/** Delete a key if present; resolve on commit or reject on IndexedDB failure. */
const idbDelete = (key) => idbRequest("readwrite", (s) => s.delete(key));

// ── Autosave ───────────────────────────────────────────────────────────────

let timer = null;
let persistence = true;

/**
 * Cancel a pending save and write the current notebook, unless unavailable or
 * persistence is disabled. On success mark data-autosaved with the saved
 * revision. Serialisation and write failures are caught without retrying.
 */
async function flush() {
  clearTimeout(timer);
  timer = null;
  pendingSince = 0;
  if (!nb || !persistence) return;
  const saving = rev;
  try {
    await idbSet(AUTOSAVE_KEY, nb.to_json());
    document.documentElement.dataset.autosaved = String(saving);
  } catch (e) {
    console.error("[nexia] autosave failed", e);
  }
}

/** When the oldest unsaved change was made (0 = nothing pending). */
let pendingSince = 0;

/**
 * Schedule an autosave after AUTOSAVE_DELAY_MS of inactivity, capped at
 * AUTOSAVE_MAX_WAIT_MS from the first pending change. These are timer delays,
 * not a guarantee of write completion; browser scheduling can delay execution.
 * (Page-exit handlers below are a best-effort extra; IndexedDB does not guarantee a transaction
 * started during unload completes.)
 */
function schedule() {
  const now = Date.now();
  if (pendingSince === 0) pendingSince = now;
  const left = Math.max(0, pendingSince + AUTOSAVE_MAX_WAIT_MS - now);
  clearTimeout(timer);
  timer = setTimeout(flush, Math.min(AUTOSAVE_DELAY_MS, left));
}

// A pending write must not be lost when the tab is hidden or closed.
// (flush() reports its own failures, so its promise is deliberately not awaited.)
addEventListener("visibilitychange", () => {
  if (document.visibilityState === "hidden" && timer) void flush();
});
addEventListener("pagehide", () => {
  if (timer) void flush();
});

/** Summarise migration and repairs, or return an empty string when none are reported. */
function describeRepairs(r) {
  if (!r) return "";
  const parts = [];
  if (r.danglingLinks?.length)
    parts.push(`${r.danglingLinks.length} broken link(s) removed`);
  if (r.selfLinks?.length)
    parts.push(`${r.selfLinks.length} self-link(s) removed`);
  if (r.duplicateLinks)
    parts.push(`${r.duplicateLinks} duplicate link(s) merged`);
  if (r.rekeyedNotes?.length)
    parts.push(`${r.rekeyedNotes.length} note(s) re-indexed`);
  if (r.droppedDuplicateIds?.length)
    parts.push(`${r.droppedDuplicateIds.length} duplicate note(s) dropped`);
  if (r.invalidGeometry?.length)
    parts.push(`${r.invalidGeometry.length} invalid position(s) cleared`);
  const migrated = r.migrated ? `upgraded from format v${r.fromVersion}` : "";
  const all = [migrated, ...parts].filter(Boolean);
  return all.length ? `Notebook loaded: ${all.join(", ")}.` : "";
}

// ── File helpers ───────────────────────────────────────────────────────────

/** Offer `text` as a download named `filename`. */
function download(filename, text, type) {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

/** Show a file picker; resolve with the chosen File objects ([] if cancelled). */
function pick(accept, { multiple = false, directory = false } = {}) {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.multiple = multiple;
    input.webkitdirectory = directory;
    input.onchange = () => resolve(Array.from(input.files ?? []));
    input.oncancel = () => resolve([]);
    input.click();
  });
}

/** Replace filename separators and reserved punctuation; use notebook for an empty name. */
const safeName = () => (nb?.name() || "notebook").replace(/[\\/:*?"<>|]/g, "_");

// ── Externs ────────────────────────────────────────────────────────────────

const externs = {
  /**
   * Load WASM and restore the autosave, then call done with a Boot result.
   * Unreadable notebook data is backed up before its autosave key is deleted.
   * Storage access or quarantine failures disable persistence and add a notice;
   * other startup failures are reported through done with ok=false.
   */
  nx_boot: (wasmUrl, done) => {
    (async () => {
      const t0 = performance.now();
      await init({ module_or_path: wasmUrl });
      let notice = "";
      let restored = false;
      let quarantined = false;
      let saved = null;
      try {
        saved = await idbGet(AUTOSAVE_KEY);
      } catch (e) {
        persistence = false;
        notice = `Local storage is unavailable (${e?.message ?? e}); changes will not persist.`;
      }
      if (typeof saved === "string") {
        try {
          nb = WasmNotebook.from_json(saved);
          restored = true;
          notice = describeRepairs(nb.loadReport());
        } catch (e) {
          // Keep the unreadable data under a backup key, then start fresh.
          const backup = `${AUTOSAVE_KEY}.corrupt-${new Date().toISOString()}`;
          try {
            await idbSet(backup, saved);
            await idbDelete(AUTOSAVE_KEY);
            quarantined = true;
          } catch {
            persistence = false;
          }
          notice =
            `Your saved notebook could not be read (${e?.message ?? e}). ` +
            (quarantined ? `A copy was kept as "${backup}". ` : "") +
            "Starting a new notebook.";
        }
      }
      if (!nb) nb = new WasmNotebook("Untitled Notebook");
      reloadAll();
      document.documentElement.dataset.bootMs = String(
        Math.round(performance.now() - t0),
      );
      done({ ok: true, error: "", restored, quarantined, notice });
    })().catch((e) =>
      done({
        ok: false,
        error: e?.message ?? String(e),
        restored: false,
        quarantined: false,
        notice: "",
      }),
    );
  },
  /** Schedule an autosave; write failures are caught by flush. */
  nx_autosave: () => schedule(),
  /** Download notebook JSON; return a failed outcome on serialisation or download errors. */
  nx_save_file: () => {
    try {
      download(`${safeName()}.nexia.json`, nb.to_json(), "application/json");
      return ok();
    } catch (e) {
      return fail(e);
    }
  },
  /** Pick and load a notebook, refresh the read model and schedule autosave.
   * Call done with the outcome; cancellation has ok=false and an empty error.
   * Read, load and refresh failures become failed outcomes. */
  nx_open_file: (done) => {
    pick(".json,application/json")
      .then(async ([file]) => {
        if (!file) return done({ ok: false, error: "", notice: "", id: "" });
        const next = WasmNotebook.from_json(await file.text());
        nb = next;
        reloadAll();
        schedule();
        done(ok("", describeRepairs(nb.loadReport())));
      })
      .catch((e) => done(fail(e)));
  },
  /** Download all notes in one Markdown file; return a failed outcome on errors. */
  nx_export_markdown: () => {
    try {
      const files = nb.export_markdown();
      const bundle = files
        .map((f) => `<!-- ${f.name} -->\n\n${f.content}`)
        .join("\n\n---\n\n");
      download(`${safeName()}-vault.md`, bundle, "text/markdown");
      return ok();
    } catch (e) {
      return fail(e);
    }
  },
  /** Download an OPML outline; return a failed outcome on errors. */
  nx_export_opml: () => {
    try {
      download(`${safeName()}.opml`, nb.export_opml(), "text/x-opml");
      return ok();
    } catch (e) {
      return fail(e);
    }
  },
  /** Replace the notebook with the chosen directory’s .md files and schedule autosave.
   * Call done with the outcome; no .md files means ok=false with an empty error.
   * Read, import and refresh failures become failed outcomes. */
  nx_import_vault: (done) => {
    pick(".md,text/markdown", { multiple: true, directory: true })
      .then(async (chosen) => {
        const md = chosen.filter((f) => f.name.endsWith(".md"));
        if (md.length === 0)
          return done({ ok: false, error: "", notice: "", id: "" });
        const files = await Promise.all(
          md.map(async (f) => ({ name: f.name, content: await f.text() })),
        );
        nb.import_markdown_vault(files);
        reloadAll();
        schedule();
        done(ok("", `Imported ${files.length} note(s).`));
      })
      .catch((e) => done(fail(e)));
  },
  /** Replace the notebook, refresh the read model and schedule autosave; return an outcome. */
  nx_reset: (name) => {
    try {
      nb = new WasmNotebook(name);
      reloadAll();
      schedule();
      return ok();
    } catch (e) {
      return fail(e);
    }
  },

  /** Return the read-model revision used to invalidate UI caches. */
  nx_rev: () => rev,
  /** Return the notebook name, or an empty string before boot. */
  nx_name: () => nb?.name() ?? "",
  /** Return the number of notes in the read model. */
  nx_count: () => notes.size,
  /** Return the cached notes in title order, breaking ties by id. */
  nx_notes: () => sortedNotes(),
  /** Whether the read model contains this note id. */
  nx_has: (id) => notes.has(id),
  /** Return the cached note, or an empty unplaced note for an unknown id. */
  nx_note: (id) => notes.get(id) ?? NO_NOTE,
  /** Return incoming note ids, or an empty array if the core call fails. */
  nx_backlinks: (id) => {
    try {
      return nb.backlinks(id);
    } catch {
      return [];
    }
  },
  /** Return up to limit matches in title/id order and the full match count.
   * An empty query or a core error produces { ids: [], total: 0 }. */
  nx_search: (q, limit) => {
    try {
      return nb.searchPage(q, limit);
    } catch {
      return { ids: [], total: 0 };
    }
  },
  /** Return stored agents in insertion order, or an empty array on a core error. */
  nx_agents: () => {
    try {
      return nb.agents();
    } catch {
      return [];
    }
  },
  /** Return matching note ids, or an empty array for an unknown agent or core error. */
  nx_run_agent: (id) => {
    try {
      return nb.run_agent(id);
    } catch {
      return [];
    }
  },
  /** Return fields in name order, cached for the current read-model revision.
   * Each value holds the printed result or, when ok=false, the evaluation error.
   * A failed core call is cached as an empty array. */
  nx_fields: (id) => {
    let fields = fieldCache.get(id);
    if (!fields) {
      try {
        fields = nb.evalComputedFields(id).map((f) => ({
          name: f.name,
          source: f.source,
          ok: f.error === undefined,
          value: f.error ?? f.value,
        }));
      } catch {
        fields = [];
      }
      fieldCache.set(id, fields);
    }
    return fields;
  },
  /** Evaluate a read-only formula without storing it; errors become ok=false results.
   * Blank source succeeds with an empty value without calling the core. */
  nx_preview: (id, source) => {
    if (source.trim() === "") return { name: "", source, ok: true, value: "" };
    try {
      return { name: "", source, ok: true, value: nb.evalFormula(id, source) };
    } catch (e) {
      return { name: "", source, ok: false, value: e?.message ?? String(e) };
    }
  },

  /** Create a note, using canvas coordinates only when placed is true; return a noteOp outcome. */
  nx_create: (title, placed, x, y) =>
    noteOp(() =>
      placed ? nb.create_note_at(title, x, y) : nb.create_note(title),
    ),
  /** Rename a note and refresh its cached view; return a noteOp outcome. */
  nx_set_title: (id, title) => noteOp(() => nb.update_title(id, title)),
  /** Set content and add resolved [[Title]] links; return a deltaOp outcome.
   * Removing a reference from the text does not remove its link. */
  nx_set_content: (id, content) =>
    deltaOp(() => nb.update_content(id, content), id),
  /** Set canvas coordinates and refresh the note view; return a noteOp outcome. */
  nx_move: (id, x, y) => noteOp(() => nb.move_note(id, x, y)),
  /** Delete a note and its links; return a deltaOp outcome with an empty id. */
  nx_delete: (id) => deltaOp(() => nb.delete_note(id)),
  /** Add a directed link; return a deltaOp outcome identifying the source note. */
  nx_link: (from, to) => deltaOp(() => nb.link(from, to), from),
  /** Remove a directed link; return a deltaOp outcome identifying the source note. */
  nx_unlink: (from, to) => deltaOp(() => nb.unlink(from, to), from),
  /** Store formula source without evaluating it; return a noteOp outcome. */
  nx_set_field: (id, name, source) =>
    noteOp(() => nb.setComputedField(id, name, source)),
  /** Remove a computed field if present; return a noteOp outcome. */
  nx_remove_field: (id, name) => noteOp(() => nb.removeComputedField(id, name)),
  /** Store a query, invalidate derived caches and schedule autosave; return an outcome. */
  nx_add_agent: (name, query) => {
    try {
      nb.add_agent(name, query);
      touched();
      schedule();
      return ok();
    } catch (e) {
      return fail(e);
    }
  },
  /** Remove an agent if present, invalidate caches and schedule autosave; return an outcome. */
  nx_remove_agent: (id) => {
    try {
      nb.remove_agent(id);
      touched();
      schedule();
      return ok();
    } catch (e) {
      return fail(e);
    }
  },

  /** Test for a substring after lowercasing both strings; an empty needle matches. */
  nx_contains_ci: (h, n) => h.toLowerCase().includes(n.toLowerCase()),
  /** Return the first n Unicode code points; negative n omits that many from the end. */
  nx_prefix: (s, n) => Array.from(s).slice(0, n).join(""),
  /** Count Unicode code points, which may differ from visible characters. */
  nx_char_count: (s) => Array.from(s).length,
  /** Remove leading and trailing JavaScript whitespace. */
  nx_trim: (s) => s.trim(),
  /** Render a number, rounding non-integers to three decimal places and dropping trailing zeroes.
   * Large or small results may use exponential notation. */
  nx_num: (x) =>
    Number.isInteger(x) ? String(x) : String(Number(x.toFixed(3))),
};

for (const [name, fn] of Object.entries(externs)) {
  if (name in globalThis)
    throw new Error(`nexia_host: global ${name} is already defined`);
  globalThis[name] = fn;
}
