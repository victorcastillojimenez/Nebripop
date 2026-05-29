// crates/api/src/web/filters.rs

pub fn format_decimal(s: &rust_decimal::Decimal, fmt: &str) -> askama::Result<String> {
    let precision = fmt
        .strip_prefix("%.")
        .and_then(|f| f.strip_suffix('f'))
        .and_then(|f| f.parse::<usize>().ok())
        .unwrap_or(2);
    Ok(format!("{:.1$}", s, precision))
}

pub fn default(opt: &Option<String>, default_val: &str) -> askama::Result<String> {
    match opt {
        Some(v) => Ok(v.clone()),
        None => Ok(default_val.to_string()),
    }
}

// Askama parser helpers to avoid complex expressions inside HTML files
pub fn empty_string_option() -> Option<String> {
    None
}

pub fn empty_f64_option() -> Option<f64> {
    None
}

pub fn empty_string_vec() -> Vec<String> {
    Vec::new()
}

pub fn contains(vec: &[String], val: &str) -> bool {
    vec.iter().any(|item| item == val)
}

