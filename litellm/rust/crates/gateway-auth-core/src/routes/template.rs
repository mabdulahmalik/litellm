use std::sync::OnceLock;

use litellm_starlette::{Convertor, RouteTemplate, TemplatePart};

/// A route template matched as `RouteChecks._route_matches_pattern` matches it: every parameter
/// takes one or more characters up to a `/`, whatever its convertor, and a `{name:path}` one or
/// more of any character, or of any but `:` when the template has a `:` after it. Literal text is
/// compared exactly.
#[derive(Debug)]
pub(super) struct Template(RouteTemplate);

impl Template {
    pub(super) fn compile(template: &str) -> Self {
        Self(RouteTemplate::parse(template))
    }

    pub(super) fn matches(&self, route: &str) -> bool {
        matches_parts(self.0.parts(), route)
    }
}

fn matches_parts(parts: &[TemplatePart], route: &str) -> bool {
    let Some((part, rest)) = parts.split_first() else {
        return route.is_empty() || route == "\n";
    };
    let candidates = |limit: usize| {
        route
            .char_indices()
            .map(|(index, _)| index)
            .chain([route.len()])
            .filter(move |index| *index >= 1 && *index <= limit)
    };
    let limit = match part {
        TemplatePart::Literal(literal) => {
            return route
                .strip_prefix(&**literal)
                .is_some_and(|after| matches_parts(rest, after));
        }
        TemplatePart::Param {
            convertor: Convertor::Path,
            colon_after: true,
        } => route.find(':').unwrap_or(route.len()),
        TemplatePart::Param {
            convertor: Convertor::Path,
            colon_after: false,
        } => route.len(),
        TemplatePart::Param { .. } => route.find('/').unwrap_or(route.len()),
    };
    candidates(limit).any(|index| matches_parts(rest, &route[index..]))
}

/// Route templates named in the core, compiled on first use.
#[derive(Debug)]
pub(crate) struct RouteSet {
    templates: &'static [&'static str],
    compiled: OnceLock<Box<[Template]>>,
}

impl RouteSet {
    pub(crate) const fn new(templates: &'static [&'static str]) -> Self {
        Self {
            templates,
            compiled: OnceLock::new(),
        }
    }

    pub(crate) fn matches(&self, route: &str) -> bool {
        self.compiled
            .get_or_init(|| {
                self.templates
                    .iter()
                    .map(|template| Template::compile(template))
                    .collect()
            })
            .iter()
            .any(|template| template.matches(route))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::exact("/models", "/models", true)]
    #[case::exact_is_whole_route("/models", "/models/x", false)]
    #[case::segment("/files/{file_id}", "/files/abc", true)]
    #[case::segment_needs_a_char("/files/{file_id}/content", "/files//content", false)]
    #[case::segment_stops_at_slash("/files/{file_id}", "/files/a/b", false)]
    #[case::path_spans_slashes(
        "/v1beta/models/{model_name:path}:generateContent",
        "/v1beta/models/a/b:generateContent",
        true
    )]
    #[case::path_needs_a_char("/engines/{model:path}/completions", "/engines//completions", false)]
    #[case::path_spans_newlines(
        "/engines/{model:path}/chat/completions",
        "/engines/a\nb/chat/completions",
        true
    )]
    #[case::path_takes_colons_without_one_after(
        "/v1/mcp/server/{path:path}",
        "/v1/mcp/server/a:b",
        true
    )]
    #[case::path_stops_before_the_templates_colon(
        "/models/{model_name:path}:generateContent",
        "/models/a:b:generateContent",
        false
    )]
    #[case::path_never_swallows_another_verb(
        "/models/{model_name:path}:countTokens",
        "/models/a:generateContent:countTokens",
        false
    )]
    #[case::path_before_colon_needs_a_char(
        "/models/{model_name:path}:generateContent",
        "/models/:generateContent",
        false
    )]
    #[case::literal_after_param(
        "/claude-code/plugins/{plugin_name}/archive/{digest}.zip",
        "/claude-code/plugins/p/archive/d.zip",
        true
    )]
    #[case::dot_is_literal(
        "/claude-code/plugins/{plugin_name}/archive/{digest}.zip",
        "/claude-code/plugins/p/archive/dxzip",
        false
    )]
    #[case::question_mark_is_literal("/realtime?{model}", "/realtime?gpt", true)]
    #[case::trailing_newline("/models", "/models\n", true)]
    #[case::star_is_literal_in_templates("/a/*", "/a/b", false)]
    #[case::host_template_drops_port("chat_completion:80", "chat_completion", true)]
    #[case::int_param_is_one_segment("/custom/{item_id:int}", "/custom/5", true)]
    #[case::int_param_takes_any_segment("/custom/{item_id:int}", "/custom/abc", true)]
    #[case::int_param_stops_at_slash("/custom/{item_id:int}", "/custom/5/x", false)]
    #[case::float_param_is_one_segment("/x/{value:float}/y", "/x/1.5/y", true)]
    #[case::uuid_param_is_one_segment(
        "/x/{id:uuid}",
        "/x/1b4e28ba-2fa1-11d2-883f-0016d3cca427",
        true
    )]
    fn templates_match_like_route_checks(
        #[case] template: &str,
        #[case] route: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(Template::compile(template).matches(route), expected);
    }
}
