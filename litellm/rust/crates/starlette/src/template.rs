/// A starlette path convertor, by the name a template spells it with. `{name}` is `Str`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumString)]
pub enum Convertor {
    #[strum(serialize = "str")]
    Str,
    #[strum(serialize = "path")]
    Path,
    #[strum(serialize = "int")]
    Int,
    #[strum(serialize = "float")]
    Float,
    #[strum(serialize = "uuid")]
    Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplatePart {
    Literal(Box<str>),
    /// `colon_after`: the template has a `:` anywhere after this parameter, as the Google
    /// `{model:path}:generateContent` routes do.
    Param {
        convertor: Convertor,
        colon_after: bool,
    },
}

/// A route template such as `/threads/{thread_id}` or a pass-through `path`. Every string parses:
/// a convertor starlette does not know reads as `Str`, and a host template such as
/// `chat_completion:80` drops its port.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(from = "String")]
pub struct RouteTemplate(Box<[TemplatePart]>);

impl From<String> for RouteTemplate {
    fn from(template: String) -> Self {
        Self::parse(&template)
    }
}

impl RouteTemplate {
    pub fn parse(template: &str) -> Self {
        let is_host = !template.starts_with('/');
        let mut parts = Vec::new();
        let mut rest = template;
        while let Some((before, convertor, after)) = next_param(rest) {
            parts.push(TemplatePart::Literal(before.into()));
            parts.push(TemplatePart::Param {
                convertor: convertor
                    .and_then(|name| name.parse().ok())
                    .unwrap_or(Convertor::Str),
                colon_after: after.contains(':'),
            });
            rest = after;
        }
        let tail = if is_host {
            rest.split(':').next().unwrap_or_default()
        } else {
            rest
        };
        parts.push(TemplatePart::Literal(tail.into()));
        Self(
            parts
                .into_iter()
                .filter(|part| !matches!(part, TemplatePart::Literal(text) if text.is_empty()))
                .collect(),
        )
    }

    pub fn parts(&self) -> &[TemplatePart] {
        &self.0
    }

    /// The route matches the regex `compile_path` builds from the template. Each convertor takes
    /// what its starlette regex takes: `str` one or more of anything but `/`, `path` zero or more
    /// of anything but a newline, `int` digits, `float` digits with an optional fraction, and
    /// `uuid` 32 hex digits with optional dashes between the groups. Like `re.match` against a
    /// pattern ending in `$`, a trailing newline is allowed.
    pub fn matches(&self, route: &str) -> bool {
        matches_compiled(&self.0, route)
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The next `{name}` or `{name:convertor}`, as starlette's `PARAM_REGEX` finds it: the text before
/// it, its convertor and the text after it.
fn next_param(text: &str) -> Option<(&str, Option<&str>, &str)> {
    text.match_indices('{').find_map(|(start, _)| {
        let end = start + text[start..].find('}')?;
        let inner = &text[start + 1..end];
        let (name, convertor) = match inner.split_once(':') {
            Some((name, convertor)) => (name, Some(convertor)),
            None => (inner, None),
        };
        (is_identifier(name) && convertor.is_none_or(is_identifier))
            .then(|| (&text[..start], convertor, &text[end + 1..]))
    })
}

fn matches_compiled(parts: &[TemplatePart], route: &str) -> bool {
    let Some((part, rest)) = parts.split_first() else {
        return route.is_empty() || route == "\n";
    };
    let convertor = match part {
        TemplatePart::Literal(literal) => {
            return route
                .strip_prefix(&**literal)
                .is_some_and(|after| matches_compiled(rest, after));
        }
        TemplatePart::Param { convertor, .. } => *convertor,
    };
    convertor_ends(convertor, route).any(|end| matches_compiled(rest, &route[end..]))
}

/// Every length of `route`'s start the convertor's regex can match.
fn convertor_ends(convertor: Convertor, route: &str) -> Box<dyn Iterator<Item = usize> + '_> {
    let boundaries = move |limit: usize, min: usize| {
        route
            .char_indices()
            .map(|(index, _)| index)
            .chain([route.len()])
            .filter(move |index| *index >= min && *index <= limit)
    };
    let digits = |text: &str| text.bytes().take_while(u8::is_ascii_digit).count();
    match convertor {
        Convertor::Str => Box::new(boundaries(route.find('/').unwrap_or(route.len()), 1)),
        Convertor::Path => Box::new(boundaries(route.find('\n').unwrap_or(route.len()), 0)),
        Convertor::Int => Box::new(1..=digits(route)),
        Convertor::Float => {
            let whole = digits(route);
            let fraction = route[whole..]
                .strip_prefix('.')
                .map(digits)
                .unwrap_or_default();
            Box::new((1..=whole).chain((whole > 0 && fraction > 0).then(|| whole + 1 + fraction)))
        }
        Convertor::Uuid => Box::new(uuid_len(route).into_iter()),
    }
}

/// `[0-9a-fA-F]{8}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{12}`. A dash can
/// never be a hex digit, so the match length is unique.
fn uuid_len(route: &str) -> Option<usize> {
    let bytes = route.as_bytes();
    [8, 4, 4, 4, 12]
        .iter()
        .enumerate()
        .try_fold(0, |at, (group, &width)| {
            let at = match (group > 0, bytes.get(at)) {
                (true, Some(b'-')) => at + 1,
                _ => at,
            };
            bytes
                .get(at..at + width)
                .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))
                .map(|_| at + width)
        })
}
