use std::borrow::Cow;
use std::path::Path;

use toml::Spanned;
use toml::de::{DeArray, DeTable, DeValue};

use ruff_db::diagnostic::Diagnostic;
use ruff_python_ast::TomlSourceType;
use ruff_text_size::{TextRange, TextSize};

use crate::Locator;
use crate::checkers::ast::LintContext;
use crate::fix::{FixResult, fix_file};
use crate::linter::{FixTable, MAX_ITERATIONS, report_failed_to_converge_error};
use crate::registry::Rule;
use crate::rules::ruff::rules::{invalid_pyproject_toml, rule_codes_in_selectors};
use crate::settings::LinterSettings;
use crate::settings::types::UnsafeFixes;

pub struct TomlFixerResult<'a> {
    pub diagnostics: Vec<Diagnostic>,
    pub transformed: Cow<'a, str>,
    pub fixed: FixTable,
}

pub fn lint_toml(
    path: &Path,
    contents: &str,
    settings: &LinterSettings,
    source_type: TomlSourceType,
) -> Vec<Diagnostic> {
    let context = LintContext::new(path, contents, settings);

    let document = DeTable::parse(contents);

    if context.is_rule_enabled(Rule::RuleCodesInSelectors)
        && let Ok(document) = &document
    {
        rule_codes_in_selectors(&context, document.get_ref(), source_type);
    }

    if source_type.is_pyproject() && context.is_rule_enabled(Rule::InvalidPyprojectToml) {
        invalid_pyproject_toml(&context, document);
    }

    context.into_diagnostics()
}

/// Generate [`Diagnostic`]s for a TOML configuration file, iteratively fixing until stable.
pub fn lint_fix_toml<'a>(
    path: &Path,
    source: &'a str,
    settings: &LinterSettings,
    source_type: TomlSourceType,
    unsafe_fixes: UnsafeFixes,
) -> TomlFixerResult<'a> {
    let mut diagnostics = lint_toml(path, source, settings, source_type);
    let mut transformed = Cow::Borrowed(source);
    let mut fixed = FixTable::default();
    let mut iterations = 0;

    loop {
        let locator = Locator::new(transformed.as_ref());
        let Some(FixResult { code, fixes, .. }) = fix_file(&diagnostics, &locator, unsafe_fixes)
        else {
            return TomlFixerResult {
                diagnostics,
                transformed,
                fixed,
            };
        };

        if iterations >= MAX_ITERATIONS {
            report_failed_to_converge_error(path, transformed.as_ref(), &diagnostics);
            return TomlFixerResult {
                diagnostics,
                transformed,
                fixed,
            };
        }

        for (id, code, count) in fixes.iter() {
            *fixed.entry(id).or_default(code) += count;
        }

        transformed = Cow::Owned(code);
        iterations += 1;

        diagnostics = lint_toml(path, transformed.as_ref(), settings, source_type);
    }
}

/// Options whose value is an array of rule selectors.
///
/// For example:
///
/// ```toml
/// select = ["F401"]
/// ```
const ARRAY_SELECTOR_OPTIONS: &[&str] = &[
    "select",
    "extend-select",
    "fixable",
    "extend-fixable",
    "ignore",
    "extend-ignore",
    "unfixable",
    "extend-unfixable",
    "extend-safe-fixes",
    "extend-unsafe-fixes",
];

/// Options whose value is a table mapping file patterns to arrays of rule selectors.
///
/// For example:
///
/// ```toml
/// per-file-ignores = { "*.py" = ["F401"] }
/// ```
const TABLE_SELECTOR_OPTIONS: &[&str] = &["per-file-ignores", "extend-per-file-ignores"];

/// A rule selector string written in a Ruff configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSelector<'a> {
    /// The option that contains the selector, such as `select` or `per-file-ignores`.
    pub option: &'static str,
    /// Whether the option is nested under the `lint` table.
    pub in_lint_table: bool,
    /// The selector as written, without the surrounding quotes.
    pub value: &'a str,
    /// The range of the selector in the source, excluding the surrounding quotes.
    ///
    /// ```toml
    /// [lint]
    /// select = ["F401"]
    ///            ^^^^
    /// ```
    pub range: TextRange,
}

/// Collect every rule selector string in a `pyproject.toml` or `ruff.toml` document.
///
/// For a `pyproject.toml`, only the `[tool.ruff]` table and its `lint` subtable are searched.
/// For a `ruff.toml`, the top level and its `lint` subtable are searched. Selectors in other
/// tables, such as `[tool.ruff.format]` or `[tool.other]`, are never returned.
///
/// Other source types return no selectors.
pub fn config_selectors<'a>(
    document: &'a DeTable<'_>,
    source: &str,
    source_type: TomlSourceType,
) -> Vec<ConfigSelector<'a>> {
    let ruff = match source_type {
        TomlSourceType::Pyproject => document
            .get("tool")
            .and_then(|tool| tool.get_ref().get("ruff"))
            .and_then(|ruff| ruff.get_ref().as_table()),
        TomlSourceType::Ruff => Some(document),
        _ => None,
    };

    let mut selectors = Vec::new();

    let Some(ruff) = ruff else {
        return selectors;
    };

    collect_selectors(ruff, source, false, &mut selectors);

    if let Some(lint) = ruff.get("lint").and_then(|lint| lint.get_ref().as_table()) {
        collect_selectors(lint, source, true, &mut selectors);
    }

    selectors
}

fn collect_selectors<'a>(
    table: &'a DeTable<'_>,
    source: &str,
    in_lint_table: bool,
    selectors: &mut Vec<ConfigSelector<'a>>,
) {
    for &option in ARRAY_SELECTOR_OPTIONS {
        if let Some(value) = table.get(option)
            && let DeValue::Array(values) = value.get_ref()
        {
            collect_selector_array(values, source, option, in_lint_table, selectors);
        }
    }

    for &option in TABLE_SELECTOR_OPTIONS {
        if let Some(value) = table.get(option)
            && let DeValue::Table(per_file) = value.get_ref()
        {
            for values in per_file.values() {
                if let Some(values) = values.get_ref().as_array() {
                    collect_selector_array(values, source, option, in_lint_table, selectors);
                }
            }
        }
    }
}

fn collect_selector_array<'a>(
    values: &'a DeArray<'_>,
    source: &str,
    option: &'static str,
    in_lint_table: bool,
    selectors: &mut Vec<ConfigSelector<'a>>,
) {
    for value in values {
        let Some(selector) = value.get_ref().as_str() else {
            continue;
        };
        let Some(range) = string_content_range(value, source) else {
            continue;
        };
        selectors.push(ConfigSelector {
            option,
            in_lint_table,
            value: selector,
            range,
        });
    }
}

/// Return the range of a TOML string's contents, excluding its quotes.
///
/// The span reported by the TOML parser covers the whole string literal, including its quotes,
/// which may be single (`'F401'`), double (`"F401"`), or tripled (`"""F401"""`).
fn string_content_range(value: &Spanned<DeValue<'_>>, source: &str) -> Option<TextRange> {
    let span = value.span();
    let literal = source.get(span.clone())?;
    let quote = literal.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let quote_len = literal.len() - literal.trim_start_matches(quote).len();
    // An empty string like `""` is a single run of quote characters, so cap the quote length at
    // half of the literal.
    let quote_len = quote_len.min(literal.len() / 2);
    Some(TextRange::new(
        TextSize::try_from(span.start + quote_len).ok()?,
        TextSize::try_from(span.end - quote_len).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::TomlSourceType;
    use ruff_text_size::TextRange;
    use toml::de::DeTable;

    use super::config_selectors;

    /// Return each selector as `(option, in_lint_table, value, source text at its range)`.
    fn selectors(source: &str, source_type: TomlSourceType) -> Vec<(&str, bool, String, String)> {
        let document = DeTable::parse(source).expect("test TOML should be valid");
        config_selectors(document.get_ref(), source, source_type)
            .into_iter()
            .map(|selector| {
                (
                    selector.option,
                    selector.in_lint_table,
                    selector.value.to_string(),
                    source[selector.range.start().to_usize()..selector.range.end().to_usize()]
                        .to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn pyproject_lint_table() {
        let source = r#"
[tool.ruff.lint]
select = ["E4", 'F401', """D213"""]
ignore = ["ALL"]
"#;
        assert_eq!(
            selectors(source, TomlSourceType::Pyproject),
            [
                ("select", true, "E4".into(), "E4".into()),
                ("select", true, "F401".into(), "F401".into()),
                ("select", true, "D213".into(), "D213".into()),
                ("ignore", true, "ALL".into(), "ALL".into()),
            ]
        );
    }

    #[test]
    fn dotted_keys_and_inline_tables() {
        let source = r#"
[tool.ruff]
extend-select = ["UP"]
lint.extend-ignore = ["D"]
lint.per-file-ignores = { "tests/**" = ["S101"] }
"#;
        assert_eq!(
            selectors(source, TomlSourceType::Pyproject),
            [
                ("extend-select", false, "UP".into(), "UP".into()),
                ("extend-ignore", true, "D".into(), "D".into()),
                ("per-file-ignores", true, "S101".into(), "S101".into()),
            ]
        );
    }

    #[test]
    fn ruff_toml_top_level() {
        let source = r#"
fixable = ["F"]

[lint.extend-per-file-ignores]
"__init__.py" = ["F401", "unused-import"]
"#;
        assert_eq!(
            selectors(source, TomlSourceType::Ruff),
            [
                ("fixable", false, "F".into(), "F".into()),
                (
                    "extend-per-file-ignores",
                    true,
                    "F401".into(),
                    "F401".into()
                ),
                (
                    "extend-per-file-ignores",
                    true,
                    "unused-import".into(),
                    "unused-import".into()
                ),
            ]
        );
    }

    #[test]
    fn ignores_unrelated_tables() {
        let source = r#"
[tool.other]
select = ["F401"]

[tool.ruff.format]
select = ["F401"]

[tool.ruff.lint.isort]
known-first-party = ["F401"]
"#;
        assert!(selectors(source, TomlSourceType::Pyproject).is_empty());
        assert!(selectors(source, TomlSourceType::Unrecognized).is_empty());
    }

    #[test]
    fn empty_and_non_string_values() {
        let source = r#"
[lint]
select = ["", 1, ["F401"]]
"#;
        let document = DeTable::parse(source).expect("test TOML should be valid");
        let found = config_selectors(document.get_ref(), source, TomlSourceType::Ruff);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].value, "");
        assert_eq!(found[0].range.len(), TextRange::default().len());
    }
}
