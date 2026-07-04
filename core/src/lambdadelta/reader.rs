// SPDX-License-Identifier: MPL-2.0
//! The λδ reader: source text → [`Value`] forms (spec §1).
//!
//! Clojure-flavoured surface syntax. The reader is **total** — it never panics
//! on any input; malformed source yields a structured [`LdError::Read`]. Code is
//! data, so the reader's output *is* the thing the evaluator and macros work on.
//!
//! Supported surface:
//! - literals: `nil true false`, integers, floats, strings, `:keywords`, symbols
//! - collections: `(list)`, `[vector]`, `{map}`, `#{set}`
//! - reader sugar: `'x` `` `x `` `~x` `~@x`, `#(… % …)`, `#tag v`
//! - comments: `;` to end of line; commas are whitespace

use std::rc::Rc;

use super::error::{LdError, LdResult};
use super::value::Value;

/// Read every top-level form from `src`.
pub fn read_all(src: &str) -> LdResult<Vec<Value>> {
    let mut r = Reader::new(src);
    let mut forms = Vec::new();
    loop {
        r.skip_ws();
        if r.at_end() {
            break;
        }
        forms.push(r.read_form()?);
    }
    Ok(forms)
}

/// Read exactly one form; error if there is trailing content or none.
pub fn read_one(src: &str) -> LdResult<Value> {
    let mut forms = read_all(src)?;
    match forms.len() {
        1 => Ok(forms.pop().unwrap()),
        0 => Err(LdError::Read {
            msg: "expected a form, found none".to_string(),
            pos: 0,
        }),
        n => Err(LdError::Read {
            msg: format!("expected one form, found {n}"),
            pos: 0,
        }),
    }
}

struct Reader {
    chars: Vec<char>,
    pos: usize,
}

impl Reader {
    fn new(src: &str) -> Self {
        Reader {
            chars: src.chars().collect(),
            pos: 0,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    /// Skip whitespace (commas count), and `;` line comments.
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c == ';' {
                while let Some(c) = self.peek() {
                    self.pos += 1;
                    if c == '\n' {
                        break;
                    }
                }
            } else if c.is_whitespace() || c == ',' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn read_form(&mut self) -> LdResult<Value> {
        self.skip_ws();
        let start = self.pos;
        let c = self.peek().ok_or_else(|| LdError::Read {
            msg: "unexpected end of input".to_string(),
            pos: start,
        })?;
        match c {
            '(' => self.read_delimited('(', ')').map(Value::list),
            '[' => self.read_delimited('[', ']').map(Value::vector),
            '{' => self.read_map(),
            ')' | ']' | '}' => Err(LdError::Read {
                msg: format!("unexpected '{c}'"),
                pos: start,
            }),
            '"' => self.read_string(),
            '\'' => self.read_wrapped("quote"),
            '`' => self.read_wrapped("quasiquote"),
            '~' => {
                if self.peek2() == Some('@') {
                    self.pos += 2;
                    let form = self.read_form()?;
                    Ok(Value::list(vec![Value::sym("unquote-splicing"), form]))
                } else {
                    self.read_wrapped("unquote")
                }
            }
            '#' => self.read_dispatch(),
            ':' => self.read_keyword(),
            _ => self.read_atom(),
        }
    }

    /// `(quote x)` / `(quasiquote x)` / `(unquote x)` sugar.
    fn read_wrapped(&mut self, head: &str) -> LdResult<Value> {
        self.pos += 1; // consume the sugar char
        let form = self.read_form()?;
        Ok(Value::list(vec![Value::sym(head), form]))
    }

    fn read_delimited(&mut self, open: char, close: char) -> LdResult<Vec<Value>> {
        let start = self.pos;
        debug_assert_eq!(self.peek(), Some(open));
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => {
                    return Err(LdError::Read {
                        msg: format!("unclosed '{open}'"),
                        pos: start,
                    })
                }
                Some(c) if c == close => {
                    self.pos += 1;
                    return Ok(items);
                }
                Some(c) if c == ')' || c == ']' || c == '}' => {
                    return Err(LdError::Read {
                        msg: format!("mismatched delimiter: expected '{close}', found '{c}'"),
                        pos: self.pos,
                    })
                }
                Some(_) => items.push(self.read_form()?),
            }
        }
    }

    fn read_map(&mut self) -> LdResult<Value> {
        let start = self.pos;
        let items = self.read_delimited('{', '}')?;
        if items.len() % 2 != 0 {
            return Err(LdError::Read {
                msg: "map literal needs an even number of forms".to_string(),
                pos: start,
            });
        }
        let mut pairs: Vec<(Value, Value)> = Vec::with_capacity(items.len() / 2);
        let mut it = items.into_iter();
        while let (Some(k), Some(v)) = (it.next(), it.next()) {
            // Last key wins on duplicates (keeps the map a proper function).
            if let Some(slot) = pairs.iter_mut().find(|(ek, _)| *ek == k) {
                slot.1 = v;
            } else {
                pairs.push((k, v));
            }
        }
        Ok(Value::Map(Rc::new(pairs)))
    }

    fn read_string(&mut self) -> LdResult<Value> {
        let start = self.pos;
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.bump() {
                None => {
                    return Err(LdError::Read {
                        msg: "unterminated string".to_string(),
                        pos: start,
                    })
                }
                Some('"') => return Ok(Value::str(out)),
                Some('\\') => {
                    let esc = self.bump().ok_or_else(|| LdError::Read {
                        msg: "unterminated string escape".to_string(),
                        pos: self.pos,
                    })?;
                    match esc {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'u' => out.push(self.read_unicode_escape()?),
                        other => {
                            return Err(LdError::Read {
                                msg: format!("invalid string escape: \\{other}"),
                                pos: self.pos,
                            })
                        }
                    }
                }
                Some(c) => out.push(c),
            }
        }
    }

    fn read_unicode_escape(&mut self) -> LdResult<char> {
        let start = self.pos;
        let mut code: u32 = 0;
        for _ in 0..4 {
            let d = self.bump().ok_or_else(|| LdError::Read {
                msg: "unterminated \\u escape".to_string(),
                pos: start,
            })?;
            let digit = d.to_digit(16).ok_or_else(|| LdError::Read {
                msg: format!("invalid \\u escape digit: {d}"),
                pos: self.pos,
            })?;
            code = code * 16 + digit;
        }
        char::from_u32(code).ok_or_else(|| LdError::Read {
            msg: format!("\\u escape is not a valid character: {code:04x}"),
            pos: start,
        })
    }

    /// Handle everything that begins with `#`: `#{set}`, `#(fn shorthand)`, and
    /// `#tag value` tagged literals.
    fn read_dispatch(&mut self) -> LdResult<Value> {
        let start = self.pos;
        self.pos += 1; // consume '#'
        match self.peek() {
            Some('{') => self.read_delimited('{', '}').map(dedup_set),
            Some('(') => {
                let body = self.read_delimited('(', ')').map(Value::list)?;
                Ok(expand_fn_shorthand(body))
            }
            Some(c) if is_symbol_start(c) => {
                let tag = self.read_symbol_token();
                self.skip_ws();
                let value = self.read_form()?;
                Ok(Value::Tagged {
                    tag: Rc::from(tag.as_str()),
                    value: Rc::new(value),
                })
            }
            _ => Err(LdError::Read {
                msg: "unsupported dispatch after '#'".to_string(),
                pos: start,
            }),
        }
    }

    fn read_keyword(&mut self) -> LdResult<Value> {
        let start = self.pos;
        self.pos += 1; // consume ':'
        let name = self.read_symbol_token();
        if name.is_empty() {
            return Err(LdError::Read {
                msg: "empty keyword".to_string(),
                pos: start,
            });
        }
        Ok(Value::kw(name.as_str()))
    }

    /// A bare atom: number, `nil`/`true`/`false`, or a symbol.
    fn read_atom(&mut self) -> LdResult<Value> {
        let tok = self.read_symbol_token();
        debug_assert!(!tok.is_empty(), "read_atom on non-token char");
        Ok(match tok.as_str() {
            "nil" => Value::Nil,
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => parse_number(&tok).unwrap_or_else(|| Value::sym(tok.as_str())),
        })
    }

    /// Consume a maximal run of symbol constituent characters.
    fn read_symbol_token(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_symbol_char(c) {
                s.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        s
    }
}

/// Characters that terminate a token / are reader-significant.
fn is_delimiter(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '(' | ')' | '[' | ']' | '{' | '}' | '"' | ';' | '\'' | '`' | '~' | ','
        )
}

fn is_symbol_char(c: char) -> bool {
    !is_delimiter(c)
}

fn is_symbol_start(c: char) -> bool {
    is_symbol_char(c) && c != '#'
}

/// Parse a token as an integer or float, or `None` if it is a symbol.
///
/// Guard: only tokens whose numeric intent is unambiguous are considered, so
/// symbols like `nan`, `inf`, `-`, and `->` are never mistaken for numbers
/// (Rust's float parser would otherwise accept "nan"/"inf").
fn parse_number(tok: &str) -> Option<Value> {
    let bytes = tok.as_bytes();
    let first = *bytes.first()?;
    let looks_numeric = first.is_ascii_digit()
        || ((first == b'-' || first == b'+' || first == b'.')
            && bytes.get(1).is_some_and(|b| b.is_ascii_digit()));
    if !looks_numeric {
        return None;
    }
    if let Ok(i) = tok.parse::<i64>() {
        return Some(Value::Int(i));
    }
    if let Ok(x) = tok.parse::<f64>() {
        if x.is_finite() {
            return Some(Value::Float(x));
        }
    }
    None
}

/// Build a set from read items, dropping later duplicates (by value equality).
fn dedup_set(items: Vec<Value>) -> Value {
    let mut members: Vec<Value> = Vec::with_capacity(items.len());
    for item in items {
        if !members.contains(&item) {
            members.push(item);
        }
    }
    Value::Set(Rc::new(members))
}

/// Expand `#(… % …)` into `(fn [%1 … %N] …)`.
///
/// `%` is an alias for `%1`; `%N` sets the arity. With no `%` the result is a
/// zero-argument function. Nested `#(…)` is not supported (a documented L0
/// limitation) — the inner `%` would be ambiguous.
fn expand_fn_shorthand(body: Value) -> Value {
    let rewritten = rewrite_pct(&body);
    let max = max_pct(&rewritten);
    let params: Vec<Value> = (1..=max).map(|n| Value::sym(format!("%{n}"))).collect();
    Value::list(vec![Value::sym("fn"), Value::vector(params), rewritten])
}

/// Replace the bare symbol `%` with `%1` throughout a form.
fn rewrite_pct(v: &Value) -> Value {
    match v {
        Value::Symbol(s) if s.as_ref() == "%" => Value::sym("%1"),
        Value::List(items) => Value::list(items.iter().map(rewrite_pct).collect()),
        Value::Vector(items) => Value::vector(items.iter().map(rewrite_pct).collect()),
        Value::Set(items) => Value::Set(Rc::new(items.iter().map(rewrite_pct).collect())),
        Value::Map(pairs) => Value::Map(Rc::new(
            pairs
                .iter()
                .map(|(k, val)| (rewrite_pct(k), rewrite_pct(val)))
                .collect(),
        )),
        other => other.clone(),
    }
}

/// Highest `%N` positional referenced (0 if none).
fn max_pct(v: &Value) -> usize {
    match v {
        Value::Symbol(s) => s
            .strip_prefix('%')
            .and_then(|rest| rest.parse::<usize>().ok())
            .unwrap_or(0),
        Value::List(items) | Value::Vector(items) | Value::Set(items) => {
            items.iter().map(max_pct).max().unwrap_or(0)
        }
        Value::Map(pairs) => pairs
            .iter()
            .map(|(k, val)| max_pct(k).max(max_pct(val)))
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}
