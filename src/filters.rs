use heck::{ToKebabCase, ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};
use minijinja::value::Kwargs;
use minijinja::Environment;

/// Register all 13 built-in filters on the given Environment.
pub fn register_all(env: &mut Environment) {
    env.add_filter("snakecase", filter_snakecase);
    env.add_filter("camelcase", filter_camelcase);
    env.add_filter("pascalcase", filter_pascalcase);
    env.add_filter("kebabcase", filter_kebabcase);
    env.add_filter("upper", filter_upper);
    env.add_filter("lower", filter_lower);
    env.add_filter("capitalize", filter_capitalize);
    env.add_filter("replace", filter_replace);
    env.add_filter("pluralize", filter_pluralize);
    env.add_filter("singularize", filter_singularize);
    env.add_filter("quote", filter_quote);
    env.add_filter("indent", filter_indent);
    env.add_filter("join", filter_join);
}

fn filter_snakecase(value: String) -> String {
    value.to_snake_case()
}

fn filter_camelcase(value: String) -> String {
    value.to_lower_camel_case()
}

fn filter_pascalcase(value: String) -> String {
    value.to_upper_camel_case()
}

fn filter_kebabcase(value: String) -> String {
    value.to_kebab_case()
}

fn filter_upper(value: String) -> String {
    value.to_uppercase()
}

fn filter_lower(value: String) -> String {
    value.to_lowercase()
}

fn filter_capitalize(value: String) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            upper + &chars.as_str().to_lowercase()
        }
    }
}

fn filter_replace(value: String, from: String, to: String) -> String {
    value.replace(&from, &to)
}

fn filter_pluralize(value: String) -> String {
    pluralizer::pluralize(&value, 2, false)
}

fn filter_singularize(value: String) -> String {
    pluralizer::pluralize(&value, 1, false)
}

fn filter_quote(value: String) -> String {
    format!("\"{}\"", value)
}

/// Indent each line by `width` spaces. By default, indents ALL lines including the first.
/// Use `indent(N, first=false)` to skip the first line.
fn filter_indent(
    value: String,
    width: usize,
    kwargs: Kwargs,
) -> Result<String, minijinja::Error> {
    let indent_first: bool = if kwargs.has("first") {
        kwargs.get::<bool>("first")?
    } else {
        true
    };
    kwargs.assert_all_used()?;

    let prefix = " ".repeat(width);
    let mut result = String::new();

    for (i, line) in value.lines().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        if (i == 0 && !indent_first) || line.is_empty() {
            result.push_str(line);
        } else {
            result.push_str(&prefix);
            result.push_str(line);
        }
    }

    // Preserve trailing newline if original had one
    if value.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

/// Join an iterable with a separator string.
fn filter_join(value: Vec<minijinja::Value>, separator: String) -> String {
    value
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(&separator)
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use minijinja::Environment;

    fn render(template: &str, ctx: &minijinja::Value) -> String {
        let mut env = Environment::new();
        super::register_all(&mut env);
        env.render_str(template, ctx).unwrap()
    }

    // ── AC-3.5: snakecase ──────────────────────────────────────

    #[test]
    fn ac_3_5_snakecase() {
        let ctx = minijinja::context! { name => "BookingService" };
        assert_eq!(render("{{ name | snakecase }}", &ctx), "booking_service");
    }

    // ── AC-3.6: camelcase ──────────────────────────────────────

    #[test]
    fn ac_3_6_camelcase() {
        let ctx = minijinja::context! { name => "booking_service" };
        assert_eq!(render("{{ name | camelcase }}", &ctx), "bookingService");
    }

    // ── AC-3.7: pascalcase ─────────────────────────────────────

    #[test]
    fn ac_3_7_pascalcase() {
        let ctx = minijinja::context! { name => "booking_service" };
        assert_eq!(render("{{ name | pascalcase }}", &ctx), "BookingService");
    }

    // ── AC-3.8: kebabcase ──────────────────────────────────────

    #[test]
    fn ac_3_8_kebabcase() {
        let ctx = minijinja::context! { name => "BookingService" };
        assert_eq!(render("{{ name | kebabcase }}", &ctx), "booking-service");
    }

    // ── AC-3.9: replace ────────────────────────────────────────

    #[test]
    fn ac_3_9_replace() {
        let ctx = minijinja::context! { path => "a.b.c" };
        assert_eq!(render("{{ path | replace('.', '/') }}", &ctx), "a/b/c");
    }

    // ── AC-3.10: pluralize ─────────────────────────────────────

    #[test]
    fn ac_3_10_pluralize() {
        let ctx = minijinja::context! { word => "hotel" };
        assert_eq!(render("{{ word | pluralize }}", &ctx), "hotels");
    }

    // ── AC-3.11: singularize ───────────────────────────────────

    #[test]
    fn ac_3_11_singularize() {
        let ctx = minijinja::context! { word => "hotels" };
        assert_eq!(render("{{ word | singularize }}", &ctx), "hotel");
    }

    // ── AC-3.12: quote ─────────────────────────────────────────

    #[test]
    fn ac_3_12_quote() {
        let ctx = minijinja::context! { word => "hello" };
        assert_eq!(render("{{ word | quote }}", &ctx), "\"hello\"");
    }

    // ── AC-3.13: indent ────────────────────────────────────────

    #[test]
    fn ac_3_13_indent_all_lines() {
        let ctx = minijinja::context! { text => "line1\nline2\nline3" };
        let result = render("{{ text | indent(4) }}", &ctx);
        assert_eq!(result, "    line1\n    line2\n    line3");
    }

    #[test]
    fn ac_3_13_indent_skip_first() {
        let ctx = minijinja::context! { text => "line1\nline2\nline3" };
        let result = render("{{ text | indent(4, first=false) }}", &ctx);
        assert_eq!(result, "line1\n    line2\n    line3");
    }

    // ── AC-3.14: join ──────────────────────────────────────────

    #[test]
    fn ac_3_14_join() {
        let ctx = minijinja::context! { items => vec!["a", "b", "c"] };
        assert_eq!(render("{{ items | join(\", \") }}", &ctx), "a, b, c");
    }

    // ── AC-3.4: All 13 filters registered ──────────────────────

    #[test]
    fn ac_3_4_all_filters_registered() {
        let mut env = Environment::new();
        super::register_all(&mut env);

        let filters = [
            ("snakecase", "{{ x | snakecase }}", "foo_bar"),
            ("camelcase", "{{ x | camelcase }}", "fooBar"),
            ("pascalcase", "{{ x | pascalcase }}", "FooBar"),
            ("kebabcase", "{{ x | kebabcase }}", "foo-bar"),
            ("upper", "{{ x | upper }}", "FOO_BAR"),
            ("lower", "{{ x | lower }}", "foo_bar"),
            ("capitalize", "{{ x | capitalize }}", "Foo_bar"),
        ];

        for (name, tmpl, expected) in &filters {
            let ctx = minijinja::context! { x => "foo_bar" };
            let result = env.render_str(tmpl, ctx).unwrap();
            assert_eq!(&result, expected, "filter {name} failed");
        }
    }

    // ── Additional filter tests ────────────────────────────────

    #[test]
    fn upper_lower_filters() {
        let ctx = minijinja::context! { x => "Hello World" };
        assert_eq!(render("{{ x | upper }}", &ctx), "HELLO WORLD");
        assert_eq!(render("{{ x | lower }}", &ctx), "hello world");
    }

    #[test]
    fn capitalize_filter() {
        let ctx = minijinja::context! { x => "hELLO" };
        assert_eq!(render("{{ x | capitalize }}", &ctx), "Hello");
    }

    // ── Insta snapshot: all 13 filters applied to "BookingService" ──

    #[test]
    fn snapshot_all_filters() {
        let input = "BookingService";
        let ctx = minijinja::context! {
            s => input,
            arr => vec!["one", "two", "three"],
            multi => "line1\nline2\nline3",
        };

        let results = vec![
            ("snakecase", render("{{ s | snakecase }}", &ctx)),
            ("camelcase", render("{{ s | camelcase }}", &ctx)),
            ("pascalcase", render("{{ s | pascalcase }}", &ctx)),
            ("kebabcase", render("{{ s | kebabcase }}", &ctx)),
            ("upper", render("{{ s | upper }}", &ctx)),
            ("lower", render("{{ s | lower }}", &ctx)),
            ("capitalize", render("{{ s | capitalize }}", &ctx)),
            ("replace", render("{{ s | replace('Service', 'Handler') }}", &ctx)),
            ("pluralize", render("{{ s | pluralize }}", &ctx)),
            ("singularize", render("{{ s | singularize }}", &ctx)),
            ("quote", render("{{ s | quote }}", &ctx)),
            ("indent", render("{{ multi | indent(4) }}", &ctx)),
            ("join", render("{{ arr | join(', ') }}", &ctx)),
        ];

        let snapshot: String = results
            .iter()
            .map(|(name, val)| format!("{name}: {val}"))
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!(snapshot);
    }
}
