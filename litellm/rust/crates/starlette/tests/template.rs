use litellm_starlette::{Convertor, RouteTemplate, TemplatePart};
use rstest::rstest;

fn literal(text: &str) -> TemplatePart {
    TemplatePart::Literal(text.into())
}

fn param(convertor: Convertor, colon_after: bool) -> TemplatePart {
    TemplatePart::Param {
        convertor,
        colon_after,
    }
}

#[rstest]
#[case::plain("/models", vec![literal("/models")])]
#[case::segment("/files/{file_id}", vec![literal("/files/"), param(Convertor::Str, false)])]
#[case::int("/custom/{item_id:int}", vec![literal("/custom/"), param(Convertor::Int, false)])]
#[case::float("/x/{v:float}/y", vec![literal("/x/"), param(Convertor::Float, false), literal("/y")])]
#[case::uuid("/x/{id:uuid}", vec![literal("/x/"), param(Convertor::Uuid, false)])]
#[case::unknown_convertor_is_str("/x/{id:slug}", vec![literal("/x/"), param(Convertor::Str, false)])]
#[case::path_before_colon(
    "/models/{m:path}:generateContent",
    vec![literal("/models/"), param(Convertor::Path, true), literal(":generateContent")]
)]
#[case::not_a_param("/x/{1}", vec![literal("/x/{1}")])]
#[case::host_drops_port("chat_completion:80", vec![literal("chat_completion")])]
fn templates_parse(#[case] template: &str, #[case] expected: Vec<TemplatePart>) {
    assert_eq!(RouteTemplate::parse(template).parts(), expected.as_slice());
}

/// Expected values from starlette 1.3.1: `bool(compile_path(template)[0].match(route))`.
#[rstest]
#[case::exact("/models", "/models", true)]
#[case::trailing_newline("/models", "/models\n", true)]
#[case::str_stops_at_slash("/files/{file_id}", "/files/a/b", false)]
#[case::str_takes_a_newline("/files/{file_id}", "/files/a\nb", true)]
#[case::path_may_be_empty(
    "/v1beta/models/{m:path}:generateContent",
    "/v1beta/models/:generateContent",
    true
)]
#[case::path_swallows_colons(
    "/v1beta/models/{m:path}:generateContent",
    "/v1beta/models/a:b:generateContent",
    true
)]
#[case::path_stops_at_a_newline(
    "/engines/{m:path}/completions",
    "/engines/a\nb/completions",
    false
)]
#[case::int_takes_digits("/x/{id:int}", "/x/42", true)]
#[case::int_refuses_letters("/x/{id:int}", "/x/4a", false)]
#[case::float_with_fraction("/x/{v:float}", "/x/1.5", true)]
#[case::float_without_fraction("/x/{v:float}", "/x/1", true)]
#[case::float_needs_fraction_digits("/x/{v:float}", "/x/1.", false)]
#[case::uuid_with_dashes("/x/{id:uuid}", "/x/1b4e28ba-2fa1-11d2-883f-0016D3CCA427", true)]
#[case::uuid_without_dashes("/x/{id:uuid}", "/x/1b4e28ba2fa111d2883f0016d3cca427", true)]
#[case::uuid_too_short("/x/{id:uuid}", "/x/1b4e28ba-2fa1-11d2-883f-0016d3cca42", false)]
#[case::dot_is_literal("/a/{x}.zip", "/a/bxzip", false)]
fn templates_match_like_compile_path(
    #[case] template: &str,
    #[case] route: &str,
    #[case] expected: bool,
) {
    assert_eq!(RouteTemplate::parse(template).matches(route), expected);
}
