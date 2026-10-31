#![cfg(feature = "sprig")]

use std::sync::Arc;

use gotmpl::{Template, TemplateError, Value, sprig, tmap};

fn render(tmpl: &str, data: &Value) -> String {
    Template::new("test")
        .parse(tmpl)
        .unwrap()
        .execute_to_string(data)
        .unwrap()
}

fn render_err(tmpl: &str, data: &Value) -> TemplateError {
    Template::new("test")
        .parse(tmpl)
        .unwrap()
        .execute_to_string(data)
        .unwrap_err()
}

#[test]
fn sprig_funcs_map_contains_all_eight() {
    let m = sprig::funcs();
    for name in [
        "default",
        "toJson",
        "list",
        "append",
        "values",
        "split",
        "atoi",
        "regexMatch",
    ] {
        assert!(m.contains_key(name), "missing function {name}");
    }
}

#[test]
fn test_default() {
    assert_eq!(
        render(
            r#"{{ .Value | default "fallback" }}"#,
            &tmap! { "Value" => "custom" }
        ),
        "custom"
    );
    assert_eq!(
        render(
            r#"{{ .Value | default "fallback" }}"#,
            &tmap! { "Value" => "" }
        ),
        "fallback"
    );
    assert_eq!(
        render(
            r#"{{ .Missing | default "fallback" }}"#,
            &tmap! { "Value" => "x" }
        ),
        "fallback"
    );
    assert_eq!(
        render(r#"{{ .Flag | default true }}"#, &tmap! { "Flag" => false }),
        "true"
    );
    assert_eq!(
        render(r#"{{ .Count | default 42 }}"#, &tmap! { "Count" => 0i64 }),
        "42"
    );
    assert_eq!(render(r#"{{ default "only" }}"#, &Value::Nil), "only");

    assert!(matches!(
        render_err(r#"{{ default }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 1,
            got: 0,
        } if name == "default"
    ));
    assert!(matches!(
        render_err(r#"{{ default "a" "b" "c" }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 2,
            got: 3,
        } if name == "default"
    ));
}

#[test]
fn test_to_json() {
    let data = tmap! {
        "Config" => tmap! {
            "enabled" => true,
            "count" => 3i64,
            "u" => 7u64,
            "ratio" => 1.5f64,
            "empty" => Value::Nil,
            "items" => vec!["a\"b", "c\\d", "e\nf\tg\r\x08\x0c\x01<html>&"],
        }
    };
    assert_eq!(
        render(r#"{{ .Config | toJson }}"#, &data),
        r#"{"count":3,"empty":null,"enabled":true,"items":["a\"b","c\\d","e\nf\tg\r\b\f\u0001<html>&"],"ratio":1.5,"u":7}"#
    );

    // Non-finite floats and functions serialize as null
    let special = tmap! {
        "nan" => f64::NAN,
        "inf" => f64::INFINITY,
        "fn" => Value::Function(Arc::new(|_| Ok(Value::Nil))),
    };
    assert_eq!(
        render(r#"{{ . | toJson }}"#, &special),
        r#"{"fn":null,"inf":null,"nan":null}"#
    );

    assert!(matches!(
        render_err(r#"{{ toJson }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 1,
            got: 0,
        } if name == "toJson"
    ));
}

#[test]
fn test_list_and_append() {
    assert_eq!(
        render(
            r#"{{ $items := list "a" "b" }}{{ $items = append $items "c" }}{{ $items | toJson }}"#,
            &Value::Nil
        ),
        r#"["a","b","c"]"#
    );
    assert_eq!(render(r#"{{ list | toJson }}"#, &Value::Nil), "[]");
    assert_eq!(
        render(r#"{{ append .Missing "first" | toJson }}"#, &tmap! {}),
        r#"["first"]"#
    );

    assert!(matches!(
        render_err(r#"{{ append "not-a-list" "x" }}"#, &Value::Nil),
        TemplateError::TypeMismatch {
            expected: "list",
            got: "string",
        }
    ));
    assert!(matches!(
        render_err(r#"{{ append (list 1) }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 2,
            got: 1,
        } if name == "append"
    ));
}

#[test]
fn test_values() {
    let data = tmap! {
        "Tags" => tmap! {
            "b" => "two",
            "a" => "one",
            "c" => "three",
        }
    };
    assert_eq!(
        render(r#"{{ values .Tags | toJson }}"#, &data),
        r#"["one","two","three"]"#
    );

    assert!(matches!(
        render_err(r#"{{ values "not-a-map" }}"#, &Value::Nil),
        TemplateError::TypeMismatch {
            expected: "map",
            got: "string",
        }
    ));
    assert!(matches!(
        render_err(r#"{{ values }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 1,
            got: 0,
        } if name == "values"
    ));
}

#[test]
fn test_split() {
    let data = tmap! { "Version" => "2.14.3" };
    assert_eq!(
        render(
            r#"{{ $parts := split "." .Version }}{{ $parts._0 }}-{{ $parts._1 }}-{{ $parts._2 }}"#,
            &data
        ),
        "2-14-3"
    );

    // Empty separator splits into UTF-8 characters
    assert_eq!(
        render(
            r#"{{ $chars := split "" "aé" }}{{ $chars._0 }}:{{ $chars._1 }}"#,
            &Value::Nil
        ),
        "a:é"
    );

    assert!(matches!(
        render_err(r#"{{ split 1 "abc" }}"#, &Value::Nil),
        TemplateError::TypeMismatch {
            expected: "string",
            got: "int",
        }
    ));
    assert!(matches!(
        render_err(r#"{{ split "." 123 }}"#, &Value::Nil),
        TemplateError::TypeMismatch {
            expected: "string",
            got: "int",
        }
    ));
    assert!(matches!(
        render_err(r#"{{ split "." }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 2,
            got: 1,
        } if name == "split"
    ));
}

#[test]
fn test_atoi() {
    let data = tmap! { "Version" => "2.14.3" };
    assert_eq!(
        render(
            r#"{{ $parts := split "." .Version }}{{ $major := $parts._0 | atoi }}{{ if ge $major 2 }}v2+{{ end }}"#,
            &data
        ),
        "v2+"
    );
    assert_eq!(render(r#"{{ "-42" | atoi }}"#, &Value::Nil), "-42");

    assert!(matches!(
        render_err(r#"{{ "abc" | atoi }}"#, &Value::Nil),
        TemplateError::Exec(ref msg) if msg.contains("atoi: invalid integer")
    ));
    assert!(matches!(
        render_err(r#"{{ 123 | atoi }}"#, &Value::Nil),
        TemplateError::TypeMismatch {
            expected: "string",
            got: "int",
        }
    ));
    assert!(matches!(
        render_err(r#"{{ atoi }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 1,
            got: 0,
        } if name == "atoi"
    ));
}

#[test]
fn test_regex_match() {
    let data = tmap! { "Version" => "v2.14.3" };
    assert_eq!(
        render(
            r#"{{ if regexMatch "^v[0-9]+\\." .Version }}match{{ end }}"#,
            &data
        ),
        "match"
    );
    assert_eq!(
        render(
            r#"{{ if regexMatch "^v[0-9]+\\." "2.14.3" }}match{{ else }}no{{ end }}"#,
            &Value::Nil
        ),
        "no"
    );

    assert!(matches!(
        render_err(r#"{{ regexMatch "[" "abc" }}"#, &Value::Nil),
        TemplateError::Exec(ref msg) if msg.contains("regexMatch:")
    ));
    assert!(matches!(
        render_err(r#"{{ regexMatch 123 "abc" }}"#, &Value::Nil),
        TemplateError::TypeMismatch {
            expected: "string",
            got: "int",
        }
    ));
    assert!(matches!(
        render_err(r#"{{ regexMatch "a" }}"#, &Value::Nil),
        TemplateError::ArgCount {
            ref name,
            expected: 2,
            got: 1,
        } if name == "regexMatch"
    ));
}
