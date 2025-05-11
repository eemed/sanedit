pub(crate) mod change;
pub(crate) mod matcher;
pub(crate) mod text;
pub(crate) mod window;

use std::cmp;

pub(crate) fn is_yes(input: &str) -> bool {
    input.eq_ignore_ascii_case("y")
        || input.eq_ignore_ascii_case("ye")
        || input.eq_ignore_ascii_case("yes")
}

pub(crate) fn to_human_readable(num: f64) -> String {
    let num = num.abs();
    let units = ["B", "K", "M", "G", "T"];
    if num < 1_f64 {
        return format!("{}{}", num, "B");
    }
    let delimiter = 1024_f64;
    let exponent = cmp::min(
        (num.ln() / delimiter.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );
    let pretty_bytes = format!("{:.2}", num / delimiter.powi(exponent))
        .parse::<f64>()
        .unwrap()
        * 1_f64;
    let unit = units[exponent as usize];
    format!("{}{}", pretty_bytes, unit)
}
