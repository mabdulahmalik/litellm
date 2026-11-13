use litellm_python_compat::lenient::{
    IsTrue, Lenient, StrBool, Truthy, parse_pydantic_bool, parse_str_bool,
};
use rstest::rstest;
use serde::Deserialize;
use serde_json::{Value, json};
use serde_with::serde_as;

#[serde_as]
#[derive(Deserialize)]
struct Flag(#[serde_as(as = "IsTrue")] bool);

/// Expected values from CPython: `value is True` for the JSON value loaded with `json.loads`.
#[rstest]
#[case::true_(json!(true), true)]
#[case::false_(json!(false), false)]
#[case::string_true(json!("true"), false)]
#[case::one(json!(1), false)]
#[case::null(json!(null), false)]
#[case::list_with_true(json!([true]), false)]
fn is_true_is_only_the_boolean_true(#[case] value: Value, #[case] expected: bool) {
    assert_eq!(serde_json::from_value::<Flag>(value).unwrap().0, expected);
}

#[serde_as]
#[derive(Deserialize)]
struct Bool(#[serde_as(as = "Truthy")] bool);

/// Expected values from CPython: `bool(value)` for the JSON value loaded with `json.loads`.
#[rstest]
#[case::true_(json!(true), true)]
#[case::false_(json!(false), false)]
#[case::zero(json!(0), false)]
#[case::one(json!(1), true)]
#[case::zero_float(json!(0.0), false)]
#[case::half(json!(0.5), true)]
#[case::empty_string(json!(""), false)]
#[case::string_false(json!("false"), true)]
#[case::null(json!(null), false)]
#[case::empty_list(json!([]), false)]
#[case::list_of_false(json!([false]), true)]
#[case::empty_mapping(json!({}), false)]
#[case::mapping(json!({"a": null}), true)]
fn truthy_is_python_bool(#[case] value: Value, #[case] expected: bool) {
    assert_eq!(serde_json::from_value::<Bool>(value).unwrap().0, expected);
}

#[rstest]
#[case::list(json!(["/a", "/b/*"]), Lenient::Valid(["/a".to_owned(), "/b/*".to_owned()].into()))]
#[case::empty(json!([]), Lenient::Valid([].into()))]
#[case::bare_string(json!("/a/*"), Lenient::Malformed)]
#[case::number(json!(5), Lenient::Malformed)]
#[case::mapping(json!({"a": 1}), Lenient::Malformed)]
#[case::non_string_entry(json!(["/a", 5, "/b/*"]), Lenient::Malformed)]
#[case::null_entry(json!(["/a", null]), Lenient::Malformed)]
fn a_value_of_another_shape_is_malformed(
    #[case] value: Value,
    #[case] expected: Lenient<Box<[String]>>,
) {
    assert_eq!(
        serde_json::from_value::<Lenient<Box<[String]>>>(value).unwrap(),
        expected
    );
}

#[rstest]
#[case::listed(Lenient::Valid(["/a".to_owned()].into()), &["/a"][..])]
#[case::malformed(Lenient::Malformed, &[][..])]
fn items_of_a_malformed_list_are_none(
    #[case] list: Lenient<Box<[String]>>,
    #[case] expected: &[&str],
) {
    assert_eq!(list.items(), expected);
}

#[rstest]
#[case::trimmed_true(" True ", Some(true))]
#[case::control_whitespace_true("\u{1c}TRUE\u{1f}", Some(true))]
#[case::unicode_whitespace_false("\u{a0}False\u{2003}", Some(false))]
#[case::zero_width_space("true\u{200b}", None)]
#[case::yes("yes", None)]
#[case::one("1", None)]
#[case::empty("", None)]
#[case::unknown("unknown", None)]
fn boolean_tokens_follow_python_string_trimming_without_redis_tokens(
    #[case] input: &str,
    #[case] expected: Option<bool>,
) {
    assert_eq!(parse_str_bool(input), expected, "{input:?}");
}

#[rstest]
#[case::yes(" YES ", Some(true))]
#[case::on("On", Some(true))]
#[case::one("1", Some(true))]
#[case::t("t", Some(true))]
#[case::y("y", Some(true))]
#[case::control_whitespace_true("\u{1c}TRUE\u{1f}", Some(true))]
#[case::no("No", Some(false))]
#[case::off("OFF", Some(false))]
#[case::zero("0", Some(false))]
#[case::f("F", Some(false))]
#[case::n("n", Some(false))]
#[case::false_word("false", Some(false))]
#[case::two("2", None)]
#[case::empty("", None)]
#[case::unknown("maybe", None)]
fn boolean_words_follow_pydantic_lax_bool_parsing(
    #[case] input: &str,
    #[case] expected: Option<bool>,
) {
    assert_eq!(parse_pydantic_bool(input), expected, "{input:?}");
}

#[serde_as]
#[derive(Deserialize)]
struct TextFlag(#[serde_as(as = "StrBool")] bool);

#[rstest]
#[case::flag(json!(true), true)]
#[case::false_flag(json!(false), false)]
#[case::true_text(json!(" True "), true)]
#[case::python_stripped_true_text(json!("\u{1c}true"), true)]
#[case::false_text(json!("false"), false)]
#[case::other_text(json!("yes"), false)]
#[case::number(json!(1), false)]
#[case::null(json!(null), false)]
fn str_bool_is_true_only_for_true(#[case] value: Value, #[case] expected: bool) {
    assert_eq!(
        serde_json::from_value::<TextFlag>(value).unwrap().0,
        expected
    );
}
