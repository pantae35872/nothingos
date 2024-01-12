pub mod converter;
pub mod oserror;
pub mod port;

#[macro_export]
macro_rules! inline_if {
    ($condition:expr, $true_expr:expr, $false_expr:expr) => {
        if $condition {
            $true_expr
        } else {
            $false_expr
        }
    };
}

pub fn floorf64(x: f64) -> f64 {
    let integer_part = x as i64;

    if x >= 0.0 || x == integer_part as f64 {
        integer_part as f64
    } else {
        (integer_part - 1) as f64
    }
}
