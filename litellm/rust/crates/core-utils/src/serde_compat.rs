pub use litellm_python_compat::lenient::{parse_pydantic_bool, parse_str_bool};

/// `redis-py` string Booleans: only `1`, `true`, and `yes` (case-insensitive) are true.
pub fn parse_redis_bool(value: &str) -> bool {
    value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
}
