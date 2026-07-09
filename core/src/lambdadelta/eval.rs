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
use super::value::{
    base_name, is_marked, map_lookup, value_eq, Closure, Env, MultiMethod, Scope, Value,
};
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

            // Symbols resolve through the scope chain (hygiene-aware).
            Value::Symbol(s) => self.resolve_symbol(s, env),

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

    /// Resolve a symbol, honouring hygiene: a marked *free* identifier that is
    /// not bound in scope falls back to its base name in the global (definition)
    /// environment — giving referential transparency and immunity to capture by
    /// use-site locals.
    fn resolve_symbol(&self, name: &Rc<str>, env: &Env) -> LdResult<Value> {
        if let Some(v) = env.get(name) {
            return Ok(v);
        }
        if is_marked(name) {
            if let Some(v) = self.global.get(base_name(name)) {
                return Ok(v);
            }
        }
        Err(LdError::Unbound(base_name(name).to_string()))
    }

    fn eval_list(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        // `()` evaluates to itself (an empty list).
        let Some(head) = items.first() else {
            return Ok(Value::list(Vec::new()));
        };

        // Special forms and macros dispatch on the *base* name, so a
        // hygienically-marked `let`/`if`/user-macro in a template still works.
        if let Value::Symbol(s) = head {
            match base_name(s) {
                "quote" => return self.sf_quote(items),
                "if" => return self.sf_if(items, env),
                "do" => return self.sf_do(items, env),
                "let" => return self.sf_let(items, env),
                "fn" | "lambda" => return self.sf_fn(items, env),
                "def" => return self.sf_def(items, env),
                "defmacro" => return self.sf_defmacro(items, env),
                "defmulti" => return self.sf_defmulti(items, env),
                "defmethod" => return self.sf_defmethod(items, env),
                "quasiquote" => {
                    self.arity_special("quasiquote", items, 1)?;
                    return self.eval_quasi(&items[1], env, 1);
                }
                "unquote" | "unquote-splicing" => {
                    return Err(LdError::Syntax {
                        form: base_name(s).to_string(),
                        msg: "used outside of a quasiquote".to_string(),
                    });
                }
                other => {
                    if let Some(mac) = self.macros.get(other).cloned() {
                        let expanded = self.expand_macro(&mac, &items[1..])?;
                        return self.eval(&expanded, env);
                    }
                }
            }
        }

        // Ordinary application: evaluate head and arguments, then apply.
        let f = self.eval(head, env)?;
        let args = self.eval_each(&items[1..], env)?;
        self.apply(&f, &args)
    }

    // ── Macros & hygiene ─────────────────────────────────────────────────────

    /// Expand one macro call: apply the transformer to the *unevaluated*
    /// argument forms under a fresh hygiene mark, so the template's introduced
    /// symbols cannot capture.
    fn expand_macro(&mut self, mac: &Rc<Closure>, arg_forms: &[Value]) -> LdResult<Value> {
        self.tick()?;
        self.mark_counter += 1;
        let mark = self.mark_counter;
        let prev = self.current_mark.replace(mark);
        let result = self.apply_closure(mac, arg_forms);
        self.current_mark = prev;
        result
    }

    /// Expand `form` once if its head names a macro; otherwise return it as-is.
    pub fn macroexpand_1(&mut self, form: &Value) -> LdResult<Value> {
        if let Value::List(items) = form {
            if let Some(Value::Symbol(s)) = items.first() {
                if let Some(mac) = self.macros.get(base_name(s)).cloned() {
                    return self.expand_macro(&mac, &items[1..]);
                }
            }
        }
        Ok(form.clone())
    }

    /// Repeatedly expand until the head no longer names a macro.
    pub fn macroexpand(&mut self, form: &Value) -> LdResult<Value> {
        let mut cur = form.clone();
        loop {
            let next = self.macroexpand_1(&cur)?;
            if next == cur {
                return Ok(next);
            }
            cur = next;
        }
    }

    /// During macro expansion, tag a template-introduced symbol with the current
    /// hygiene mark (unless it already carries one). Outside expansion this is a
    /// no-op, so ordinary quasiquote is unchanged.
    fn mark_symbol(&self, s: &Rc<str>) -> Value {
        match self.current_mark {
            Some(m) if !is_marked(s) => {
                Value::Symbol(Rc::from(super::value::mangle(s, m).as_str()))
            }
            _ => Value::Symbol(s.clone()),
        }
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

    fn sf_defmacro(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        // (defmacro name [params] body…)
        if items.len() < 3 {
            return Err(LdError::Syntax {
                form: "defmacro".to_string(),
                msg: "expected (defmacro name [params] body…)".to_string(),
            });
        }
        let Value::Symbol(name) = &items[1] else {
            return Err(LdError::Syntax {
                form: "defmacro".to_string(),
                msg: "macro name must be a symbol".to_string(),
            });
        };
        let Value::Vector(param_vals) = &items[2] else {
            return Err(LdError::Syntax {
                form: "defmacro".to_string(),
                msg: "parameters must be a vector".to_string(),
            });
        };
        let (params, rest) = parse_params(param_vals)?;
        let body = items[3..].to_vec();
        let transformer = Rc::new(Closure {
            name: Some(name.clone()),
            params,
            rest,
            body,
            env: env.clone(),
        });
        self.macros.insert(Rc::from(base_name(name)), transformer);
        Ok(Value::Nil)
    }

    fn sf_defmulti(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        // (defmulti name dispatch-fn)
        if items.len() != 3 {
            return Err(LdError::Syntax {
                form: "defmulti".to_string(),
                msg: "expected (defmulti name dispatch-fn)".to_string(),
            });
        }
        let Value::Symbol(name) = &items[1] else {
            return Err(LdError::Syntax {
                form: "defmulti".to_string(),
                msg: "multimethod name must be a symbol".to_string(),
            });
        };
        let dispatch = self.eval(&items[2], env)?;
        let key: Rc<str> = Rc::from(base_name(name));
        let mm = Rc::new(std::cell::RefCell::new(MultiMethod {
            dispatch,
            methods: Vec::new(),
            default: None,
        }));
        self.multimethods.insert(key.clone(), mm.clone());
        // Calling `name` dispatches: run the dispatch fn on the args, pick the
        // matching method (or `:default`), and apply it to the same args.
        self.register_builtin(&key, 0, None, move |i, args| {
            let (dispatch, methods, default) = {
                let m = mm.borrow();
                (m.dispatch.clone(), m.methods.clone(), m.default.clone())
            };
            let dv = i.apply(&dispatch, args)?;
            let chosen = methods
                .iter()
                .find(|(k, _)| value_eq(k, &dv))
                .map(|(_, c)| c.clone())
                .or(default);
            match chosen {
                Some(c) => i.apply(&Value::Fn(c), args),
                None => Err(LdError::User(format!("no matching method for {dv}"))),
            }
        });
        Ok(Value::Nil)
    }

    fn sf_defmethod(&mut self, items: &[Value], env: &Env) -> LdResult<Value> {
        // (defmethod name dispatch-val [params] body…); dispatch-val `:default`
        // registers the fallback.
        if items.len() < 4 {
            return Err(LdError::Syntax {
                form: "defmethod".to_string(),
                msg: "expected (defmethod name dispatch-val [params] body…)".to_string(),
            });
        }
        let Value::Symbol(name) = &items[1] else {
            return Err(LdError::Syntax {
                form: "defmethod".to_string(),
                msg: "multimethod name must be a symbol".to_string(),
            });
        };
        let key: Rc<str> = Rc::from(base_name(name));
        let mm = self
            .multimethods
            .get(&key)
            .cloned()
            .ok_or_else(|| LdError::User(format!("defmethod: no multimethod named {key}")))?;
        let is_default = matches!(&items[2], Value::Keyword(k) if k.as_ref() == "default");
        let dispatch_val = if is_default {
            Value::Nil
        } else {
            self.eval(&items[2], env)?
        };
        let Value::Vector(param_vals) = &items[3] else {
            return Err(LdError::Syntax {
                form: "defmethod".to_string(),
                msg: "parameters must be a vector".to_string(),
            });
        };
        let (params, rest) = parse_params(param_vals)?;
        let body = items[4..].to_vec();
        let method = Rc::new(Closure {
            name: Some(name.clone()),
            params,
            rest,
            body,
            env: env.clone(),
        });
        if is_default {
            mm.borrow_mut().default = Some(method);
        } else {
            mm.borrow_mut().methods.push((dispatch_val, method));
        }
        Ok(Value::Nil)
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
                    if base_name(s) == "unquote" && items.len() == 2 {
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
            // A template-introduced symbol gets the current hygiene mark.
            Value::Symbol(s) => Ok(self.mark_symbol(s)),
            // Other atoms (keywords, numbers, …) quote as themselves.
            other => Ok(other.clone()),
        }
    }

    /// Build a sequence under quasiquote, honouring `~@` splicing.
    fn quasi_seq(&mut self, items: &[Value], env: &Env, depth: usize) -> LdResult<Vec<Value>> {
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            if let Value::List(inner) = item {
                if let Some(Value::Symbol(s)) = inner.first() {
                    if base_name(s) == "unquote-splicing" && inner.len() == 2 {
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
            Value::Symbol(s) if base_name(s) == "&" => {
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
