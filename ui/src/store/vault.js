// SPDX-License-Identifier: MPL-2.0
// Markdown-vault delivery: write one file per note via the File System Access
// API where available (Chromium), else download a single concatenated file.
// No third-party zip library — honours the Deno-only / no-npm policy.

function downloadBlob(filename, text, type) {
  const blob = new Blob([text], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

// files: [{ name, content }]. Returns "directory" | "bundle".
export async function exportMarkdownVault(files) {
  if (typeof window.showDirectoryPicker === "function") {
    try {
      const dir = await window.showDirectoryPicker({ mode: "readwrite" });
      for (const file of files) {
        const handle = await dir.getFileHandle(file.name, { create: true });
        const writable = await handle.createWritable();
        await writable.write(file.content);
        await writable.close();
      }
      return "directory";
    } catch (err) {
      if (err && err.name === "AbortError") return "cancelled";
      // Fall through to the bundle download on any picker/permission failure.
    }
  }
  const bundle = files
    .map((f) => `<!-- ${f.name} -->\n\n${f.content}`)
    .join("\n\n---\n\n");
  downloadBlob("nexia-vault.md", bundle, "text/markdown");
  return "bundle";
}

export function downloadOpml(name, text) {
  downloadBlob(`${name || "notebook"}.opml`, text, "text/x-opml");
}

// Prompt for a directory (or files) and return [{ name, content }] for .md files.
export function pickMarkdownVault() {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    input.accept = ".md,text/markdown";
    // webkitdirectory lets the user pick a whole vault folder.
    input.webkitdirectory = true;
    input.onchange = async () => {
      const chosen = Array.from(input.files || []).filter((f) => f.name.endsWith(".md"));
      if (chosen.length === 0) return resolve([]);
      const files = await Promise.all(
        chosen.map(async (f) => ({ name: f.name, content: await f.text() })),
      );
      resolve(files);
    };
    input.oncancel = () => resolve([]);
    input.click();
  });
}
