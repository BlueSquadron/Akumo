//! The Tier-1 expression engine (ADR-0018 / SPEC-D7 = "YAML + a safe expression language").
//!
//! Akumo techniques use a small, **safe, non-Turing-complete** expression language for step
//! conditions, bounded iteration, precondition/effect argument resolution, and parameter
//! templating. It is intentionally **self-contained** (no external interpreter): full control of the
//! sandbox matters for a safety-critical tool, and the surface is CEL-aligned so it can be swapped
//! for `cel-interpreter` later without changing technique authoring.
//!
//! Values are [`serde_json::Value`] (the same type facts and results use). The language has:
//! literals (number/string/bool/null), variables and `.field` access, the operators
//! `! - * / % + - < <= > >= == != && ||`, and the built-ins `has`, `size`, `contains`,
//! `starts_with`, `ends_with`, `lower`, `upper`. There is **no** assignment, loop, or I/O.

use std::collections::HashMap;

use akumo_domain::error::{AkumoError, Result};

/// The value type expressions evaluate to.
pub type Value = serde_json::Value;

/// The variable environment an expression is evaluated against (inputs, facts, step results, the
/// current principal, …).
#[derive(Clone, Debug, Default)]
pub struct Env {
    vars: HashMap<String, Value>,
}

impl Env {
    /// An empty environment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a variable (builder-style).
    pub fn with(mut self, name: impl Into<String>, value: Value) -> Self {
        self.vars.insert(name.into(), value);
        self
    }

    /// Bind a variable.
    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        self.vars.insert(name.into(), value);
    }

    /// Look up a variable.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.vars.get(name)
    }
}

/// Evaluate an expression to a value.
pub fn eval(source: &str, env: &Env) -> Result<Value> {
    let tokens = lex(source)?;
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_expr(0)?;
    if parser.pos != parser.tokens.len() {
        return Err(err("unexpected trailing tokens in expression"));
    }
    eval_expr(&expr, env)
}

/// Evaluate an expression that must yield a boolean (a step `condition`).
pub fn eval_bool(source: &str, env: &Env) -> Result<bool> {
    match eval(source, env)? {
        Value::Bool(b) => Ok(b),
        other => Err(err(&format!("condition must be boolean, got {other}"))),
    }
}

/// Resolve one templated value:
/// - `"$path"` (a bare `$` + variable path) → the referenced value, preserving its type;
/// - a string containing `${expr}` → interpolation into a string;
/// - anything else → the literal string.
pub fn resolve_value(source: &str, env: &Env) -> Result<Value> {
    let trimmed = source.trim();
    if !trimmed.contains("${") {
        if let Some(path) = trimmed.strip_prefix('$') {
            if is_path(path) {
                return eval(path, env);
            }
        }
        return Ok(Value::String(source.to_string()));
    }
    Ok(Value::String(interpolate(source, env)?))
}

/// Evaluate an expression that must yield an array (a step `for_each` collection). The caller
/// (execution engine, G9) enforces the iteration bound.
pub fn eval_array(source: &str, env: &Env) -> Result<Vec<Value>> {
    match eval(source, env)? {
        Value::Array(a) => Ok(a),
        other => Err(err(&format!("for_each expects an array, got {other}"))),
    }
}

/// Resolve every top-level string value in a parameter map against `env`.
pub fn resolve_params(
    params: &serde_json::Map<String, Value>,
    env: &Env,
) -> Result<serde_json::Map<String, Value>> {
    let mut out = serde_json::Map::new();
    for (key, value) in params {
        let resolved = match value {
            Value::String(s) => resolve_value(s, env)?,
            other => other.clone(),
        };
        out.insert(key.clone(), resolved);
    }
    Ok(out)
}

// ---- lexer -------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    True,
    False,
    Null,
    LParen,
    RParen,
    Dot,
    Comma,
    Bang,
    Star,
    Slash,
    Percent,
    Plus,
    Minus,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    NotEq,
    AndAnd,
    OrOr,
}

fn err(msg: &str) -> AkumoError {
    AkumoError::Validation(format!("expression error: {msg}"))
}

fn lex(src: &str) -> Result<Vec<Tok>> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            '.' => {
                out.push(Tok::Dot);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '*' => {
                out.push(Tok::Star);
                i += 1;
            }
            '/' => {
                out.push(Tok::Slash);
                i += 1;
            }
            '%' => {
                out.push(Tok::Percent);
                i += 1;
            }
            '+' => {
                out.push(Tok::Plus);
                i += 1;
            }
            '-' => {
                out.push(Tok::Minus);
                i += 1;
            }
            '!' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::NotEq);
                    i += 2;
                } else {
                    out.push(Tok::Bang);
                    i += 1;
                }
            }
            '=' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::EqEq);
                    i += 2;
                } else {
                    return Err(err("'=' is not an operator (use '==')"));
                }
            }
            '<' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::Le);
                    i += 2;
                } else {
                    out.push(Tok::Lt);
                    i += 1;
                }
            }
            '>' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::Ge);
                    i += 2;
                } else {
                    out.push(Tok::Gt);
                    i += 1;
                }
            }
            '&' => {
                if chars.get(i + 1) == Some(&'&') {
                    out.push(Tok::AndAnd);
                    i += 2;
                } else {
                    return Err(err("single '&' is not supported (use '&&')"));
                }
            }
            '|' => {
                if chars.get(i + 1) == Some(&'|') {
                    out.push(Tok::OrOr);
                    i += 2;
                } else {
                    return Err(err("single '|' is not supported (use '||')"));
                }
            }
            '"' | '\'' => {
                let quote = c;
                i += 1;
                let mut s = String::new();
                while i < chars.len() && chars[i] != quote {
                    // Minimal escape handling.
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        s.push(chars[i]);
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(err("unterminated string literal"));
                }
                i += 1; // closing quote
                out.push(Tok::Str(s));
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let n: f64 = text
                    .parse()
                    .map_err(|_| err(&format!("invalid number '{text}'")))?;
                out.push(Tok::Num(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                out.push(match word.as_str() {
                    "true" => Tok::True,
                    "false" => Tok::False,
                    "null" => Tok::Null,
                    _ => Tok::Ident(word),
                });
            }
            other => return Err(err(&format!("unexpected character '{other}'"))),
        }
    }
    Ok(out)
}

// ---- parser (precedence climbing) --------------------------------------------------------------

#[derive(Clone, Debug)]
enum Expr {
    Lit(Value),
    Var(String),
    Field(Box<Expr>, String),
    Call(String, Vec<Expr>),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Binding power for binary operators (higher binds tighter). Returns `None` for non-operators.
    fn binop(tok: &Tok) -> Option<(BinOp, u8)> {
        Some(match tok {
            Tok::OrOr => (BinOp::Or, 1),
            Tok::AndAnd => (BinOp::And, 2),
            Tok::EqEq => (BinOp::Eq, 3),
            Tok::NotEq => (BinOp::Ne, 3),
            Tok::Lt => (BinOp::Lt, 4),
            Tok::Le => (BinOp::Le, 4),
            Tok::Gt => (BinOp::Gt, 4),
            Tok::Ge => (BinOp::Ge, 4),
            Tok::Plus => (BinOp::Add, 5),
            Tok::Minus => (BinOp::Sub, 5),
            Tok::Star => (BinOp::Mul, 6),
            Tok::Slash => (BinOp::Div, 6),
            Tok::Percent => (BinOp::Rem, 6),
            _ => return None,
        })
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        while let Some((op, bp)) = self.peek().and_then(Self::binop) {
            if bp < min_bp {
                break;
            }
            self.advance(); // consume operator
            let right = self.parse_expr(bp + 1)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        match self.peek() {
            Some(Tok::Bang) => {
                self.advance();
                Ok(Expr::Not(Box::new(self.parse_unary()?)))
            }
            Some(Tok::Minus) => {
                self.advance();
                Ok(Expr::Neg(Box::new(self.parse_unary()?)))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        while let Some(Tok::Dot) = self.peek() {
            self.advance();
            match self.advance() {
                Some(Tok::Ident(field)) => expr = Expr::Field(Box::new(expr), field),
                _ => return Err(err("expected a field name after '.'")),
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.advance() {
            Some(Tok::Num(n)) => Ok(Expr::Lit(number_value(n))),
            Some(Tok::Str(s)) => Ok(Expr::Lit(Value::String(s))),
            Some(Tok::True) => Ok(Expr::Lit(Value::Bool(true))),
            Some(Tok::False) => Ok(Expr::Lit(Value::Bool(false))),
            Some(Tok::Null) => Ok(Expr::Lit(Value::Null)),
            Some(Tok::LParen) => {
                let inner = self.parse_expr(0)?;
                match self.advance() {
                    Some(Tok::RParen) => Ok(inner),
                    _ => Err(err("expected ')'")),
                }
            }
            Some(Tok::Ident(name)) => {
                if let Some(Tok::LParen) = self.peek() {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.parse_expr(0)?);
                            match self.advance() {
                                Some(Tok::Comma) => continue,
                                Some(Tok::RParen) => break,
                                _ => return Err(err("expected ',' or ')' in call")),
                            }
                        }
                    } else {
                        self.advance(); // consume ')'
                    }
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            other => Err(err(&format!("unexpected token {other:?}"))),
        }
    }
}

// ---- evaluator ---------------------------------------------------------------------------------

fn number_value(n: f64) -> Value {
    if n.is_finite() && n.fract() == 0.0 {
        Value::from(n as i64)
    } else {
        serde_json::Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null)
    }
}

fn eval_expr(expr: &Expr, env: &Env) -> Result<Value> {
    match expr {
        Expr::Lit(v) => Ok(v.clone()),
        Expr::Var(name) => Ok(env.get(name).cloned().unwrap_or(Value::Null)),
        Expr::Field(base, field) => {
            let base = eval_expr(base, env)?;
            match base {
                Value::Object(map) => Ok(map.get(field).cloned().unwrap_or(Value::Null)),
                _ => Ok(Value::Null),
            }
        }
        Expr::Not(inner) => {
            let v = eval_expr(inner, env)?;
            match v {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                other => Err(err(&format!("'!' expects a boolean, got {other}"))),
            }
        }
        Expr::Neg(inner) => {
            let v = eval_expr(inner, env)?;
            match v.as_f64() {
                Some(n) => Ok(number_value(-n)),
                None => Err(err("unary '-' expects a number")),
            }
        }
        Expr::Call(name, args) => eval_call(name, args, env),
        Expr::Bin(op, l, r) => eval_bin(*op, l, r, env),
    }
}

fn eval_bin(op: BinOp, l: &Expr, r: &Expr, env: &Env) -> Result<Value> {
    // Short-circuit logical operators.
    match op {
        BinOp::And => {
            return Ok(Value::Bool(as_bool(&eval_expr(l, env)?)? && as_bool(&eval_expr(r, env)?)?))
        }
        BinOp::Or => {
            return Ok(Value::Bool(as_bool(&eval_expr(l, env)?)? || as_bool(&eval_expr(r, env)?)?))
        }
        _ => {}
    }
    let lv = eval_expr(l, env)?;
    let rv = eval_expr(r, env)?;
    match op {
        BinOp::Eq => Ok(Value::Bool(value_eq(&lv, &rv))),
        BinOp::Ne => Ok(Value::Bool(!value_eq(&lv, &rv))),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => compare(op, &lv, &rv),
        BinOp::Add => add(&lv, &rv),
        BinOp::Sub => arith(&lv, &rv, |a, b| a - b),
        BinOp::Mul => arith(&lv, &rv, |a, b| a * b),
        BinOp::Div => arith(&lv, &rv, |a, b| a / b),
        BinOp::Rem => arith(&lv, &rv, |a, b| a % b),
        BinOp::And | BinOp::Or => unreachable!("handled above"),
    }
}

fn eval_call(name: &str, args: &[Expr], env: &Env) -> Result<Value> {
    let values: Result<Vec<Value>> = args.iter().map(|a| eval_expr(a, env)).collect();
    let values = values?;
    let arity = |n: usize| -> Result<()> {
        if values.len() == n {
            Ok(())
        } else {
            Err(err(&format!("{name}() expects {n} argument(s)")))
        }
    };
    match name {
        "has" => {
            arity(1)?;
            Ok(Value::Bool(!values[0].is_null()))
        }
        "size" => {
            arity(1)?;
            let n = match &values[0] {
                Value::String(s) => s.chars().count(),
                Value::Array(a) => a.len(),
                Value::Object(o) => o.len(),
                _ => return Err(err("size() expects a string, array, or object")),
            };
            Ok(Value::from(n as i64))
        }
        "contains" => {
            arity(2)?;
            Ok(Value::Bool(as_str(&values[0])?.contains(as_str(&values[1])?)))
        }
        "starts_with" => {
            arity(2)?;
            Ok(Value::Bool(as_str(&values[0])?.starts_with(as_str(&values[1])?)))
        }
        "ends_with" => {
            arity(2)?;
            Ok(Value::Bool(as_str(&values[0])?.ends_with(as_str(&values[1])?)))
        }
        "lower" => {
            arity(1)?;
            Ok(Value::String(as_str(&values[0])?.to_lowercase()))
        }
        "upper" => {
            arity(1)?;
            Ok(Value::String(as_str(&values[0])?.to_uppercase()))
        }
        _ => Err(err(&format!("unknown function '{name}'"))),
    }
}

fn as_bool(v: &Value) -> Result<bool> {
    v.as_bool().ok_or_else(|| err(&format!("expected a boolean, got {v}")))
}

fn as_str(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| err(&format!("expected a string, got {v}")))
}

fn value_eq(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

fn compare(op: BinOp, a: &Value, b: &Value) -> Result<Value> {
    let ordering = if let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) {
        x.partial_cmp(&y)
    } else if let (Some(x), Some(y)) = (a.as_str(), b.as_str()) {
        Some(x.cmp(y))
    } else {
        return Err(err("comparison requires two numbers or two strings"));
    };
    let ord = ordering.ok_or_else(|| err("values are not comparable"))?;
    use std::cmp::Ordering::*;
    let result = match op {
        BinOp::Lt => ord == Less,
        BinOp::Le => ord != Greater,
        BinOp::Gt => ord == Greater,
        BinOp::Ge => ord != Less,
        _ => unreachable!(),
    };
    Ok(Value::Bool(result))
}

fn add(a: &Value, b: &Value) -> Result<Value> {
    if let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) {
        return Ok(number_value(x + y));
    }
    if let (Some(x), Some(y)) = (a.as_str(), b.as_str()) {
        return Ok(Value::String(format!("{x}{y}")));
    }
    Err(err("'+' requires two numbers or two strings"))
}

fn arith(a: &Value, b: &Value, f: impl Fn(f64, f64) -> f64) -> Result<Value> {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => Ok(number_value(f(x, y))),
        _ => Err(err("arithmetic requires numbers")),
    }
}

// ---- templating helpers ------------------------------------------------------------------------

fn is_path(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn interpolate(source: &str, env: &Env) -> Result<String> {
    let mut out = String::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let start = i + 2;
            let end = source[start..]
                .find('}')
                .map(|off| start + off)
                .ok_or_else(|| err("unterminated '${' in template"))?;
            let inner = &source[start..end];
            out.push_str(&value_to_string(&eval(inner, env)?));
            i = end + 1;
        } else {
            let ch = source[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    Ok(out)
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn env() -> Env {
        Env::new()
            .with("user", json!("alice"))
            .with("count", json!(3))
            .with("result", json!({ "AccessKey": { "AccessKeyId": "AKIA123" } }))
            .with("flag", json!(true))
    }

    #[test]
    fn literals_and_arithmetic() {
        assert_eq!(eval("1 + 2 * 3", &env()).unwrap(), json!(7));
        assert_eq!(eval("(1 + 2) * 3", &env()).unwrap(), json!(9));
        assert_eq!(eval("10 % 3", &env()).unwrap(), json!(1));
        assert_eq!(eval("-count", &env()).unwrap(), json!(-3));
    }

    #[test]
    fn comparisons_and_logic() {
        assert!(eval_bool("count > 2 && flag", &env()).unwrap());
        assert!(!eval_bool("count > 5 || !flag", &env()).unwrap());
        assert!(eval_bool("user == 'alice'", &env()).unwrap());
        assert!(eval_bool("user != 'bob'", &env()).unwrap());
    }

    #[test]
    fn field_access_and_missing_is_null() {
        assert_eq!(
            eval("result.AccessKey.AccessKeyId", &env()).unwrap(),
            json!("AKIA123")
        );
        assert_eq!(eval("result.Missing", &env()).unwrap(), Value::Null);
        assert!(eval_bool("has(result.AccessKey)", &env()).unwrap());
        assert!(!eval_bool("has(result.Missing)", &env()).unwrap());
    }

    #[test]
    fn builtin_functions() {
        assert_eq!(eval("size(user)", &env()).unwrap(), json!(5));
        assert!(eval_bool("contains(user, 'lic')", &env()).unwrap());
        assert!(eval_bool("starts_with(result.AccessKey.AccessKeyId, 'AKIA')", &env()).unwrap());
        assert_eq!(eval("upper(user)", &env()).unwrap(), json!("ALICE"));
    }

    #[test]
    fn condition_must_be_boolean() {
        assert!(eval_bool("count", &env()).is_err());
    }

    #[test]
    fn templating_pure_path_preserves_type() {
        assert_eq!(resolve_value("$user", &env()).unwrap(), json!("alice"));
        assert_eq!(resolve_value("$count", &env()).unwrap(), json!(3));
        assert_eq!(
            resolve_value("$result.AccessKey.AccessKeyId", &env()).unwrap(),
            json!("AKIA123")
        );
    }

    #[test]
    fn templating_interpolation_and_literals() {
        assert_eq!(
            resolve_value("user-${user}-key", &env()).unwrap(),
            json!("user-alice-key")
        );
        assert_eq!(
            resolve_value("iam:CreateAccessKey", &env()).unwrap(),
            json!("iam:CreateAccessKey")
        );
    }

    #[test]
    fn resolve_params_resolves_string_values() {
        let mut params = serde_json::Map::new();
        params.insert("UserName".into(), json!("$user"));
        params.insert("Static".into(), json!("literal"));
        params.insert("Count".into(), json!(7));
        let out = resolve_params(&params, &env()).unwrap();
        assert_eq!(out["UserName"], json!("alice"));
        assert_eq!(out["Static"], json!("literal"));
        assert_eq!(out["Count"], json!(7));
    }
}
