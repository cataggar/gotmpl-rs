//! Sprig-compatible template utility functions.
//!
//! Provides a subset of functions from the Go
//! [Sprig](https://masterminds.github.io/sprig/) library commonly used in
//! Go templates:
//!
//! | Function | Signature | Description |
//! |----------|-----------|-------------|
//! | `default` | `default <defaultVal> [givenVal]` | Returns `givenVal` if provided and truthy, otherwise `defaultVal` |
//! | `toJson` | `toJson <val>` | Serializes any [`Value`] to a compact JSON string |
//! | `list` | `list [item1 item2 ...]` | Constructs a [`Value::List`] from its arguments |
//! | `append` | `append <list> <item>` | Returns a new list with `item` appended |
//! | `values` | `values <map>` | Returns a [`Value::List`] of map values in sorted key order |
//! | `split` | `split <sep> <orig>` | Splits `orig` on `sep` into a map with keys `"_0"`, `"_1"`, … |
//! | `atoi` | `atoi <str>` | Parses a base-10 string into a [`Value::Int`] |
//! | `regexMatch` | `regexMatch <pattern> <str>` | Reports whether `str` matches the regular expression `pattern` |
//!
//! When the `sprig` feature is enabled (on by default), these functions are
//! automatically included in the built-in function map registered by
//! [`Template::new`](crate::Template::new).

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::error::{Result, TemplateError};
use crate::funcs::check_args;
use crate::value::{Value, ValueFunc};

/// Returns a [`BTreeMap`] containing all Sprig-compatible template functions.
///
/// Automatically merged into the default function map when the `sprig` feature
/// is enabled.
///
/// # Examples
///
/// ```
/// use gotmpl::{Template, tmap};
///
/// let out = Template::new("t")
///     .parse(r#"{{ .Value | default "fallback" }}"#)
///     .unwrap()
///     .execute_to_string(&tmap! { "Value" => "" })
///     .unwrap();
/// assert_eq!(out, "fallback");
/// ```
pub fn funcs() -> BTreeMap<String, ValueFunc> {
    let mut m: BTreeMap<String, ValueFunc> = BTreeMap::new();
    m.insert("default".into(), Arc::new(default_fn));
    m.insert("toJson".into(), Arc::new(to_json_fn));
    m.insert("list".into(), Arc::new(list_fn));
    m.insert("append".into(), Arc::new(append_fn));
    m.insert("values".into(), Arc::new(values_fn));
    m.insert("split".into(), Arc::new(split_fn));
    m.insert("atoi".into(), Arc::new(atoi_fn));
    m.insert("regexMatch".into(), Arc::new(regex_match_fn));
    m
}

/// `default <defaultVal> [givenVal]` — returns `givenVal` when present and
/// truthy (per [`Value::is_truthy`]), otherwise `defaultVal`.
fn default_fn(args: &[Value]) -> Result<Value> {
    match args {
        [] => Err(TemplateError::ArgCount {
            name: "default".into(),
            expected: 1,
            got: 0,
        }),
        [def] => Ok(def.clone()),
        [def, given] => Ok(if given.is_truthy() {
            given.clone()
        } else {
            def.clone()
        }),
        _ => Err(TemplateError::ArgCount {
            name: "default".into(),
            expected: 2,
            got: args.len(),
        }),
    }
}

/// `toJson <val>` — serializes a [`Value`] to compact JSON.
fn to_json_fn(args: &[Value]) -> Result<Value> {
    check_args("toJson", args, 1)?;
    let mut out = String::new();
    write_json(&args[0], &mut out);
    Ok(Value::String(out.into()))
}

fn write_json(v: &Value, out: &mut String) {
    match v {
        Value::Nil | Value::Function(_) => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) => {
            let _ = write!(out, "{n}");
        }
        Value::Uint(n) => {
            let _ = write!(out, "{n}");
        }
        Value::Float(f) => {
            if f.is_finite() {
                let _ = write!(out, "{f}");
            } else {
                out.push_str("null");
            }
        }
        Value::String(s) => write_json_string(s, out),
        #[cfg(feature = "html")]
        Value::Safe { s, .. } => write_json_string(s, out),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out);
            }
            out.push(']');
        }
        Value::Map(map) => {
            out.push('{');
            for (i, (k, val)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json_string(k, out);
                out.push(':');
                write_json(val, out);
            }
            out.push('}');
        }
    }
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `list [item1 item2 ...]` — constructs a [`Value::List`] from its arguments.
fn list_fn(args: &[Value]) -> Result<Value> {
    Ok(Value::List(Arc::from(args)))
}

/// `append <list> <item>` — returns a new [`Value::List`] with `item` appended.
fn append_fn(args: &[Value]) -> Result<Value> {
    check_args("append", args, 2)?;
    match &args[0] {
        Value::List(items) => {
            let mut vec = Vec::with_capacity(items.len() + 1);
            vec.extend_from_slice(items);
            vec.push(args[1].clone());
            Ok(Value::List(vec.into()))
        }
        Value::Nil => Ok(Value::List(Arc::from([args[1].clone()]))),
        other => Err(TemplateError::TypeMismatch {
            expected: "list",
            got: other.type_name(),
        }),
    }
}

/// `values <map>` — returns a [`Value::List`] of the map's values in sorted key order.
fn values_fn(args: &[Value]) -> Result<Value> {
    check_args("values", args, 1)?;
    match &args[0] {
        Value::Map(map) => {
            let vals: Vec<Value> = map.values().cloned().collect();
            Ok(Value::List(vals.into()))
        }
        other => Err(TemplateError::TypeMismatch {
            expected: "map",
            got: other.type_name(),
        }),
    }
}

/// `split <sep> <orig>` — splits `orig` on `sep` into a [`Value::Map`] with keys `"_0"`, `"_1"`, …
fn split_fn(args: &[Value]) -> Result<Value> {
    check_args("split", args, 2)?;
    let sep = expect_str(&args[0])?;
    let orig = expect_str(&args[1])?;
    let mut map: BTreeMap<Arc<str>, Value> = BTreeMap::new();
    if sep.is_empty() {
        for (i, ch) in orig.chars().enumerate() {
            let mut buf = [0u8; 4];
            let s = ch.encode_utf8(&mut buf);
            map.insert(format!("_{i}").into(), Value::String(Arc::from(&*s)));
        }
    } else {
        for (i, part) in orig.split(sep).enumerate() {
            map.insert(format!("_{i}").into(), Value::String(Arc::from(part)));
        }
    }
    Ok(Value::Map(Arc::new(map)))
}

/// `atoi <str>` — parses a base-10 integer string into [`Value::Int`].
fn atoi_fn(args: &[Value]) -> Result<Value> {
    check_args("atoi", args, 1)?;
    let s = expect_str(&args[0])?;
    let n = s
        .parse::<i64>()
        .map_err(|_| TemplateError::Exec(format!("atoi: invalid integer {s:?}")))?;
    Ok(Value::Int(n))
}

/// `regexMatch <pattern> <str>` — reports whether `str` matches the regex `pattern`.
fn regex_match_fn(args: &[Value]) -> Result<Value> {
    check_args("regexMatch", args, 2)?;
    let pattern = expect_str(&args[0])?;
    let s = expect_str(&args[1])?;
    let re =
        regex::Regex::new(pattern).map_err(|e| TemplateError::Exec(format!("regexMatch: {e}")))?;
    Ok(Value::Bool(re.is_match(s)))
}

fn expect_str(v: &Value) -> Result<&str> {
    match v {
        Value::String(s) => Ok(s),
        #[cfg(feature = "html")]
        Value::Safe { s, .. } => Ok(s),
        other => Err(TemplateError::TypeMismatch {
            expected: "string",
            got: other.type_name(),
        }),
    }
}
