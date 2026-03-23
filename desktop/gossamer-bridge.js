// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)

/**
 * gossamer-bridge.js — Unified IPC bridge for Nexia desktop app.
 *
 * Detects the available runtime (Gossamer or browser-only) and
 * dispatches `invoke` / `listen` calls to the appropriate backend.
 *
 * Priority order:
 *   1. Gossamer (`window.__gossamer_invoke`)  — primary runtime
 *   2. Browser  (stub / error)               — development fallback
 */

/**
 * Detect which desktop runtime is available.
 * @returns {"gossamer"|"browser"}
 */
function detectRuntime() {
  if (typeof window !== 'undefined' &&
      typeof window.__gossamer_invoke === 'function') {
    return 'gossamer';
  }
  return 'browser';
}

let _runtime = null;

/**
 * Get the cached runtime identifier.
 * @returns {"gossamer"|"browser"}
 */
export function runtime() {
  if (_runtime === null) {
    _runtime = detectRuntime();
  }
  return _runtime;
}

/**
 * Invoke a backend command through whatever runtime is available.
 *
 * - On Gossamer: calls `window.__gossamer_invoke(cmd, args)`
 * - On browser:  rejects with a descriptive error
 *
 * @param {string} cmd   — The command name
 * @param {object} [args] — Optional payload object
 * @returns {Promise<any>}
 */
export function invoke(cmd, args) {
  const rt = runtime();
  if (rt === 'gossamer') {
    return window.__gossamer_invoke(cmd, args || {});
  }
  return Promise.reject(
    new Error(`No desktop runtime \u2014 "${cmd}" requires Gossamer`)
  );
}

/**
 * Listen for backend events.
 *
 * - On Gossamer: calls `window.__gossamer_listen(event, handler)`
 * - On browser:  returns a no-op unlisten function
 *
 * @param {string} event     — The event name
 * @param {function} handler — Callback receiving `{ payload: ... }`
 * @returns {Promise<function>} Unlisten function
 */
export function listen(event, handler) {
  const rt = runtime();
  if (rt === 'gossamer') {
    if (typeof window.__gossamer_listen === 'function') {
      return window.__gossamer_listen(event, handler);
    }
    console.warn('[gossamer-bridge] Gossamer event listener not available for:', event);
    return Promise.resolve(() => {});
  }
  console.warn('[gossamer-bridge] No desktop runtime \u2014 ignoring event listener for:', event);
  return Promise.resolve(() => {});
}

/**
 * Whether any desktop runtime is available.
 * @returns {boolean}
 */
export function hasDesktopRuntime() {
  return runtime() !== 'browser';
}

/**
 * Human-readable name for the current runtime.
 * @returns {string}
 */
export function runtimeName() {
  const names = { gossamer: 'Gossamer', browser: 'Browser' };
  return names[runtime()] || 'Unknown';
}
