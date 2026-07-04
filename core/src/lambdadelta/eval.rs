// SPDX-License-Identifier: MPL-2.0
//! The λδ evaluator: special forms, function application, and the sandbox
//! budget (spec §3, §6).
//!
//! The irreducible core the evaluator knows directly is exactly the spec's:
//! `quote if do let fn def` plus quasiquote/unquote/unquote-splicing. Everything
//! else is a function or (later) a hygienic macro. Collection literals evaluate
//! their elements; keywords self-evaluate and are callable as map lookups.

use std::rc::Rc;

use super::error::{LdError, LdResult};
use super::value::{map_lookup, Closure, Env, Scope, Value};
use super::Interp;

impl Interp {
    /// Charge one reduction step against the budget.
    fn tick(&mut self) -> LdResult<()> {
        self.budget.steps = self
            .budget
            .steps
            .checked_sub(1)
            .ok_or_else(|| LdError::Budget("evaluation-step limit reached".to_string()))?;
        Ok(())
    }

    /// Evaluate one form in `env`.
    pub fn eval(&mut self, form: &Value, env: &Env) -> LdResult<Value> {
        self.tick()?;
        match form {
            // Self-evaluating atoms.
            Value::Nil
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Str(_)
            | Value::Keyword(_)
            | Value::Tagged { .. }
            | Value::Fn(_)
            | Value::Builtin(_) => Ok(form.clone()),

            // Symbols resolve through the scope chain.
            Value::Symbol(s) => env.get(s).ok_or_else(|| LdError::Unbound(s.to_string())),

            // Collection literals evaluate their elements.
            Value::Vector(items) => {
                let vals = self.eval_each(items, env)?;
                Ok(Value::vector(vals))
            }
            Value::Set(items) => {
                let mut members: Vec<Value> = Vec::with_capacity(items.len());
                for item in items.iter() {
                    let v = self.eval(item, env)?;
                    if !members.contains(&v) {
                        members.push(v);
                    }
                }
                Ok(Value::Set(Rc::new(members)))
            }
            Value::Map(pairs) => {
                let mut out: Vec<(Value, Value)> = Vec::with_capacity(pairs.len());
                for (k, v) in pairs.iter() {
                    let k = self.eval(k, env)?;
                    let v = self.eval(v, env)?;
                    if let Some(slot) = out.iter_mut().find(|(ek, _)| *ek == k) {
                        slot.1 = v;
                    } else {
                        out.push((k, v));
                    }
                }
                Ok(Value::Map(Rc::new(out)))
            }

            // Lists are calls or special forms.
            Value::List(items) => self.eval_list(items, env),
        }
    }

    fn eval_each(&mut self, forms: &[Value], env: &Env) -> LdResult<Vec<Value>> {
        forms.iter().map(|f| self.eval(f, env)).collect()
    }

    fn eval_list(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        // `()` evaluates to itself (an empty list).
        let Some(head) = items.first() else {
            return Ok(Value::list(Vec::new()));
        };

        if let Value::Symbol(s) = head {
            match s.as_ref() {
                "quote" => return self.sf_quote(items),
                "if" => return self.sf_if(items, env),
                "do" => return self.sf_do(items, env),
                "let" => return self.sf_let(items, env),
                "fn" | "lambda" => return self.sf_fn(items, env),
                "def" => return self.sf_def(items, env),
                "quasiquote" => {
                    self.arity_special("quasiquote", items, 1)?;
                    return self.eval_quasi(&items[1], env, 1);
                }
                "unquote" | "unquote-splicing" => {
                    return Err(LdError::Syntax {
                        form: s.to_string(),
                        msg: "used outside of a quasiquote".to_string(),
                    });
                }
                _ => {}
            }
        }

        // Ordinary application: evaluate head and arguments, then apply.
        let f = self.eval(head, env)?;
        let args = self.eval_each(&items[1..], env)?;
        self.apply(&f, &args)
    }

    /// Apply a callable value to already-evaluated arguments.
    pub fn apply(&mut self, f: &Value, args: &[Value]) -> LdResult<Value> {
        self.tick()?;
        match f {
            Value::Builtin(b) => {
                b.check_arity(args.len())?;
                (b.func)(self, args)
            }
            Value::Fn(clo) => self.apply_closure(clo, args),
            // A keyword acts as a function: `(:k m)` → look `:k` up in map `m`,
            // with an optional default. This powers `(defmulti f :type)` and
            // `(sort-by :modified-at …)`.
            Value::Keyword(_) => self.apply_keyword(f, args),
            other => Err(LdError::NotCallable(other.to_string())),
        }
    }

    fn apply_closure(&mut self, clo: &Rc<Closure>, args: &[Value]) -> LdResult<Value> {
        // Arity check honouring an optional `& rest` parameter.
        let fixed = clo.params.len();
        let ok = if clo.rest.is_some() {
            args.len() >= fixed
        } else {
            args.len() == fixed
        };
        if !ok {
            let expected = if clo.rest.is_some() {
                format!("at least {fixed}")
            } else {
                format!("{fixed}")
            };
            return Err(LdError::Arity {
                name: clo
                    .name
                    .as_ref()
                    .map_or_else(|| "fn".to_string(), |n| n.to_string()),
                expected,
                got: args.len(),
            });
        }

        let frame = Scope::child(clo.env.clone());
        for (p, a) in clo.params.iter().zip(args) {
            frame.set(p.clone(), a.clone());
        }
        if let Some(rest) = &clo.rest {
            let extra = args[fixed..].to_vec();
            frame.set(rest.clone(), Value::vector(extra));
        }
        // A named closure can refer to itself for recursion.
        if let Some(name) = &clo.name {
            frame.set(name.clone(), Value::Fn(clo.clone()));
        }

        self.enter_depth()?;
        let mut result = Value::Nil;
        for form in &clo.body {
            match self.eval(form, &frame) {
                Ok(v) => result = v,
                Err(e) => {
                    self.leave_depth();
                    return Err(e);
                }
            }
        }
        self.leave_depth();
        Ok(result)
    }

    fn apply_keyword(&mut self, kw: &Value, args: &[Value]) -> LdResult<Value> {
        if args.is_empty() || args.len() > 2 {
            return Err(LdError::Arity {
                name: kw.to_string(),
                expected: "1..2".to_string(),
                got: args.len(),
            });
        }
        let default = args.get(1).cloned().unwrap_or(Value::Nil);
        match &args[0] {
            Value::Map(pairs) => Ok(map_lookup(pairs, kw).cloned().unwrap_or(default)),
            Value::Nil => Ok(default),
            other => Err(LdError::Type {
                op: kw.to_string(),
                expected: "map".to_string(),
                got: other.type_name().to_string(),
            }),
        }
    }

    fn enter_depth(&mut self) -> LdResult<()> {
        self.budget.depth += 1;
        if self.budget.depth > self.budget.max_depth {
            return Err(LdError::Budget(format!(
                "recursion depth limit ({}) reached",
                self.budget.max_depth
            )));
        }
        Ok(())
    }

    fn leave_depth(&mut self) {
        self.budget.depth = self.budget.depth.saturating_sub(1);
    }

    // ── Special forms ────────────────────────────────────────────────────────

    fn sf_quote(&self, items: &[Value]) -> LdResult<Value> {
        self.arity_special("quote", items, 1)?;
        Ok(items[1].clone())
    }

    fn sf_if(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        if items.len() < 3 || items.len() > 4 {
            return Err(LdError::Syntax {
                form: "if".to_string(),
                msg: "expected (if test then else?)".to_string(),
            });
        }
        let test = self.eval(&items[1], env)?;
        if test.is_truthy() {
            self.eval(&items[2], env)
        } else if let Some(else_form) = items.get(3) {
            self.eval(else_form, env)
        } else {
            Ok(Value::Nil)
        }
    }

    fn sf_do(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        let mut result = Value::Nil;
        for form in &items[1..] {
            result = self.eval(form, env)?;
        }
        Ok(result)
    }

    fn sf_let(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        let bindings = items.get(1).ok_or_else(|| LdError::Syntax {
            form: "let".to_string(),
            msg: "expected a binding vector".to_string(),
        })?;
        let Value::Vector(binds) = bindings else {
            return Err(LdError::Syntax {
                form: "let".to_string(),
                msg: "bindings must be a vector".to_string(),
            });
        };
        if binds.len() % 2 != 0 {
            return Err(LdError::Syntax {
                form: "let".to_string(),
                msg: "bindings must have an even number of forms".to_string(),
            });
        }
        // Sequential bindings: each may see the previous ones.
        let frame = Scope::child(env.clone());
        let mut i = 0;
        while i < binds.len() {
            let Value::Symbol(name) = &binds[i] else {
                return Err(LdError::Syntax {
                    form: "let".to_string(),
                    msg: "binding targets must be symbols (destructuring is deferred)".to_string(),
                });
            };
            let value = self.eval(&binds[i + 1], &frame)?;
            frame.set(name.clone(), value);
            i += 2;
        }
        let mut result = Value::Nil;
        for form in &items[2..] {
            result = self.eval(form, &frame)?;
        }
        Ok(result)
    }

    fn sf_fn(&self, items: &[Value], env: &Env) -> LdResult<Value> {
        // (fn [params] body…) or (fn name [params] body…)
        let (name, params_idx) = match items.get(1) {
            Some(Value::Symbol(n)) => (Some(n.clone()), 2),
            _ => (None, 1),
        };
        let params_form = items.get(params_idx).ok_or_else(|| LdError::Syntax {
            form: "fn".to_string(),
            msg: "expected a parameter vector".to_string(),
        })?;
        let Value::Vector(param_vals) = params_form else {
            return Err(LdError::Syntax {
                form: "fn".to_string(),
                msg: "parameters must be a vector".to_string(),
            });
        };

        let (params, rest) = parse_params(param_vals)?;
        let body = items[params_idx + 1..].to_vec();
        Ok(Value::Fn(Rc::new(Closure {
            name,
            params,
            rest,
            body,
            env: env.clone(),
        })))
    }

    fn sf_def(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        if items.len() != 3 {
            return Err(LdError::Syntax {
                form: "def".to_string(),
                msg: "expected (def name value)".to_string(),
            });
        }
        let Value::Symbol(name) = &items[1] else {
            return Err(LdError::Syntax {
                form: "def".to_string(),
                msg: "def target must be a symbol".to_string(),
            });
        };
        let value = self.eval(&items[2], env)?;
        // `def` always binds in the global (notebook) environment.
        self.global.set(name.clone(), value.clone());
        Ok(value)
    }

    // ── Quasiquote ───────────────────────────────────────────────────────────

    /// Expand a quasiquoted template. `~x` evaluates `x`; `~@xs` splices a
    /// sequence into the surrounding list/vector. Handles one level of nesting
    /// as needed by macro templates.
    fn eval_quasi(&mut self, form: &Value, env: &Env, depth: usize) -> LdResult<Value> {
        match form {
            Value::List(items) => {
                // A bare `(unquote e)` at this level evaluates `e`.
                if let Some(Value::Symbol(s)) = items.first() {
                    if s.as_ref() == "unquote" && items.len() == 2 {
                        return self.eval(&items[1], env);
                    }
                }
                let built = self.quasi_seq(items, env, depth)?;
                Ok(Value::list(built))
            }
            Value::Vector(items) => {
                let built = self.quasi_seq(items, env, depth)?;
                Ok(Value::vector(built))
            }
            // Atoms (including symbols and keywords) quote as themselves.
            other => Ok(other.clone()),
        }
    }

    /// Build a sequence under quasiquote, honouring `~@` splicing.
    fn quasi_seq(&mut self, items: &[Value], env: &Env, depth: usize) -> LdResult<Vec<Value>> {
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            if let Value::List(inner) = item {
                if let Some(Value::Symbol(s)) = inner.first() {
                    if s.as_ref() == "unquote-splicing" && inner.len() == 2 {
                        let spliced = self.eval(&inner[1], env)?;
                        match spliced {
                            Value::List(xs) | Value::Vector(xs) => out.extend(xs.iter().cloned()),
                            Value::Nil => {}
                            other => {
                                return Err(LdError::Type {
                                    op: "unquote-splicing".to_string(),
                                    expected: "sequence".to_string(),
                                    got: other.type_name().to_string(),
                                })
                            }
                        }
                        continue;
                    }
                }
            }
            out.push(self.eval_quasi(item, env, depth)?);
        }
        Ok(out)
    }

    // ── Shared helpers ───────────────────────────────────────────────────────

    fn arity_special(&self, name: &str, items: &[Value], want_args: usize) -> LdResult<()> {
        if items.len() - 1 == want_args {
            Ok(())
        } else {
            Err(LdError::Arity {
                name: name.to_string(),
                expected: want_args.to_string(),
                got: items.len() - 1,
            })
        }
    }
}

/// Fixed parameter names plus an optional `& rest` parameter.
type ParamList = (Vec<Rc<str>>, Option<Rc<str>>);

/// Parse a `fn`/closure parameter vector into fixed params plus an optional
/// `& rest` param.
fn parse_params(param_vals: &[Value]) -> LdResult<ParamList> {
    let mut params: Vec<Rc<str>> = Vec::new();
    let mut rest: Option<Rc<str>> = None;
    let mut it = param_vals.iter();
    while let Some(p) = it.next() {
        match p {
            Value::Symbol(s) if s.as_ref() == "&" => {
                let r = it.next().ok_or_else(|| LdError::Syntax {
                    form: "fn".to_string(),
                    msg: "expected a symbol after &".to_string(),
                })?;
                let Value::Symbol(name) = r else {
                    return Err(LdError::Syntax {
                        form: "fn".to_string(),
                        msg: "rest parameter must be a symbol".to_string(),
                    });
                };
                rest = Some(name.clone());
                if it.next().is_some() {
                    return Err(LdError::Syntax {
                        form: "fn".to_string(),
                        msg: "only one parameter may follow &".to_string(),
                    });
                }
            }
            Value::Symbol(s) => params.push(s.clone()),
            other => {
                return Err(LdError::Syntax {
                    form: "fn".to_string(),
                    msg: format!(
                        "parameters must be symbols (destructuring is deferred); got {}",
                        other.type_name()
                    ),
                })
            }
        }
    }
    Ok((params, rest))
}
