//! Checks that src/lib.rs declares exactly what amalgamation/libvsql.h does:
//! every type, constant (with the same value) and routine (with the same
//! parameters). A new amalgamation (scripts/update-amalgamation.sh) that adds
//! to libvsql.h fails this test until lib.rs is updated.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::path::Path;

fn read(path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Removes the /* */ comments of C.
fn strip_block_comments(source: &str) -> String {
    let mut out = String::new();
    let mut rest = source;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find("*/") {
            Some(end) => &rest[i + end + 2..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// Removes the // comments of Rust.
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A C or Rust type, such as "*mut SQLCHAR" or "SQLSMALLINT". Pointers are
/// compared without their mutability (lib.rs uses *const for the strings that
/// libvsql only reads).
fn normalize(ty: &str) -> String {
    let ty = ty.trim();
    if let Some(pointee) = ty
        .strip_prefix("*mut ")
        .or_else(|| ty.strip_prefix("*const "))
    {
        return format!("*{}", normalize(pointee));
    }
    match ty {
        "c_void" => "void".to_string(),
        ty => ty.to_string(),
    }
}

/// A C type in libvsql.h, such as "SQLCHAR *" or "int16_t", as the Rust type
/// it's declared as (normalized).
fn c_type(ty: &str) -> String {
    let ty = ty.trim();
    if let Some(pointee) = ty.strip_suffix('*') {
        return format!("*{}", c_type(pointee));
    }
    match ty {
        "unsigned char" => "u8",
        "int16_t" => "i16",
        "int32_t" => "i32",
        "int64_t" => "i64",
        "float" => "f32",
        "double" => "f64",
        ty => ty,
    }
    .to_string()
}

/// A C integer constant, such as "(-1)", "0L" or "0x00000002L".
fn c_integer(value: &str) -> i64 {
    let value = value.trim_start_matches('(').trim_end_matches(')');
    let value = value.trim_end_matches('L');
    match value.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16).unwrap(),
        None => value.parse().unwrap(),
    }
}

/// A Rust integer constant, such as "-1" or "0x0000_0002".
fn rust_integer(value: &str) -> i64 {
    let value = value.replace('_', "");
    match value.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16).unwrap(),
        None => value.parse().unwrap(),
    }
}

#[derive(Debug, Default)]
struct Declarations {
    types: BTreeMap<String, String>,
    constants: BTreeMap<String, String>,
    routines: BTreeMap<String, Vec<String>>,
}

/// The declarations in libvsql.h. Integer constants are decimal, string
/// constants are quoted. A routine is its parameters ("Name: type"), then its
/// return type.
fn header() -> Declarations {
    let mut decls = Declarations::default();
    let code = strip_block_comments(&read("amalgamation/libvsql.h"));
    for line in code.lines() {
        if let Some(define) = line.strip_prefix("#define ") {
            let mut parts = define.split_whitespace();
            let name = parts.next().unwrap();
            let Some(value) = parts.next() else {
                continue; // LIBVSQL_H
            };
            let value = if value.starts_with('"') {
                value.to_string()
            } else {
                c_integer(value).to_string()
            };
            decls.constants.insert(name.to_string(), value);
        } else if let Some(typedef) = line.strip_prefix("typedef ") {
            let typedef = typedef.trim_end_matches(';');
            let i = typedef.rfind([' ', '*']).unwrap() + 1;
            decls
                .types
                .insert(typedef[i..].to_string(), c_type(&typedef[..i]));
        }
    }

    // The routines, without the preprocessor lines (and the #defines).
    let code: Vec<_> = code.lines().filter(|line| !line.starts_with('#')).collect();
    for declaration in code.join("\n").split(';') {
        let Some(open) = declaration.find('(') else {
            continue;
        };
        let head = declaration[..open].lines().last().unwrap().trim();
        let i = head.rfind([' ', '*']).unwrap() + 1;
        let (returns, name) = (c_type(&head[..i]), &head[i..]);
        let close = declaration.rfind(')').unwrap();
        let mut signature: Vec<String> = declaration[open + 1..close]
            .split(',')
            .map(str::trim)
            .filter(|param| *param != "void")
            .map(|param| {
                let i = param.rfind([' ', '*']).unwrap() + 1;
                format!("{}: {}", &param[i..], c_type(&param[..i]))
            })
            .collect();
        signature.push(returns);
        decls.routines.insert(name.to_string(), signature);
    }
    decls
}

/// The declarations in src/lib.rs, the same as header().
fn lib() -> Declarations {
    let mut decls = Declarations::default();
    let source = strip_line_comments(&read("src/lib.rs"));
    let (before, rest) = source.split_once("extern \"C\" {").unwrap();
    let (routines, after) = rest.rsplit_once('}').unwrap();
    for item in before.split(';').chain(after.split(';')) {
        let item = item.split_whitespace().collect::<Vec<_>>().join(" ");
        let item = match item.find("pub ") {
            Some(i) => &item[i..],
            None => continue,
        };
        if let Some(constant) = item.strip_prefix("pub const ") {
            let (name, rest) = constant.split_once(':').unwrap();
            let (_, value) = rest.split_once('=').unwrap();
            let value = value.trim();
            let value = match value.split_once("b\"") {
                Some((_, string)) => {
                    format!("\"{}\"", string.split_once("\\0\"").unwrap().0)
                }
                None => rust_integer(value).to_string(),
            };
            decls.constants.insert(name.to_string(), value);
        } else if let Some(alias) = item.strip_prefix("pub type ") {
            let (name, ty) = alias.split_once('=').unwrap();
            decls.types.insert(name.trim().to_string(), normalize(ty));
        } else {
            panic!("unexpected item in src/lib.rs: {item}");
        }
    }

    for routine in routines.split(';').filter(|r| !r.trim().is_empty()) {
        let routine = routine.split_whitespace().collect::<Vec<_>>().join(" ");
        let routine = routine.strip_prefix("pub fn ").unwrap();
        let (name, rest) = routine.split_once('(').unwrap();
        let (params, returns) = rest.rsplit_once(')').unwrap();
        let mut signature: Vec<String> = params
            .split(',')
            .map(str::trim)
            .filter(|param| !param.is_empty())
            .map(|param| {
                let (name, ty) = param.split_once(':').unwrap();
                format!("{name}: {}", normalize(ty))
            })
            .collect();
        signature.push(normalize(returns.trim().trim_start_matches("->")));
        decls.routines.insert(name.to_string(), signature);
    }
    decls
}

fn compare<T: PartialEq + Debug>(
    what: &str,
    header: &BTreeMap<String, T>,
    lib: &BTreeMap<String, T>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for (name, value) in header {
        match lib.get(name) {
            None => errors.push(format!("{what} {name} isn't in src/lib.rs")),
            Some(v) if v != value => errors.push(format!(
                "{what} {name} is {v:?} in src/lib.rs, but {value:?} in libvsql.h"
            )),
            _ => {}
        }
    }
    for name in lib.keys() {
        if !header.contains_key(name) {
            errors.push(format!("{what} {name} isn't in libvsql.h"));
        }
    }
    errors
}

#[test]
fn lib_rs_matches_libvsql_h() {
    let (header, lib) = (header(), lib());

    // Make sure that the parsing found everything.
    assert!(header.types.len() >= 12, "{:?}", header.types);
    assert!(header.constants.len() >= 300, "{}", header.constants.len());
    assert!(header.routines.len() >= 60, "{:?}", header.routines.keys());

    let mut errors = compare("type", &header.types, &lib.types);
    errors.extend(compare("constant", &header.constants, &lib.constants));
    errors.extend(compare("routine", &header.routines, &lib.routines));
    assert!(errors.is_empty(), "\n{}", errors.join("\n"));
}

#[test]
fn string_constants() {
    use vsql_sys::*;
    assert_eq!(SQL_ALL_CATALOGS.to_bytes(), b"%");
    assert_eq!(SQL_ALL_SCHEMAS.to_bytes(), b"%");
    assert_eq!(SQL_ALL_TABLE_TYPES.to_bytes(), b"%");
}
