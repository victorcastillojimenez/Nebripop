// crates/api/src/web/filters.rs

pub fn format(s: &f64, fmt: &str) -> askama::Result<String> {
    Ok(format!(
        "{:.1$}",
        s,
        fmt.strip_prefix("%.").and_then(|f| f.strip_suffix('f')).and_then(|f| f.parse::<usize>().ok()).unwrap_or(2)
    ))
}
