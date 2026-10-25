use toml::de::DeTable;

use ruff_diagnostics::{Edit, Fix};
use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::TomlSourceType;

use crate::codes::Category;
use crate::toml::config_selectors;
use crate::{
    AlwaysFixableViolation, checkers::ast::LintContext, codes::Rule,
    preview::is_human_readable_names_enabled, rule_redirects::get_redirect_target,
};

/// ## What it does
///
/// Checks for any configuration files that use rule codes as selectors.
///
/// ## Why is this bad?
///
/// Human-readable rule names are easier to understand than rule codes. Using names also avoids
/// requiring readers to look up the meaning of each code.
///
/// ## Example
///
/// ```toml
/// [tool.ruff.lint]
/// select = ["F401"]
/// ```
///
/// Use instead:
///
/// ```toml
/// [tool.ruff.lint]
/// select = ["unused-import"]
/// ```
#[derive(ViolationMetadata)]
#[violation_metadata(preview_since = "0.15.22", category = Category::Pedantic)]
pub(crate) struct RuleCodesInSelectors {
    selector: &'static str,
    name: &'static str,
    in_lint_table: bool,
}

impl AlwaysFixableViolation for RuleCodesInSelectors {
    #[derive_message_formats]
    fn message(&self) -> String {
        let Self {
            selector,
            in_lint_table,
            name: _,
        } = self;
        if *in_lint_table {
            format!("Rule code used instead of name in `lint.{selector}`")
        } else {
            format!("Rule code used instead of name in `{selector}`")
        }
    }

    fn fix_title(&self) -> String {
        format!("Replace rule code with `{name}`", name = self.name)
    }
}

/// RUF201
pub(crate) fn rule_codes_in_selectors(
    context: &LintContext,
    document: &DeTable<'_>,
    source_type: TomlSourceType,
) {
    if !is_human_readable_names_enabled(context.settings().preview) {
        return;
    }

    let source = context.source_file().source_text();

    for selector in config_selectors(document, source, source_type) {
        let code = get_redirect_target(selector.value).unwrap_or(selector.value);
        let Ok(rule) = Rule::from_code(code) else {
            continue;
        };
        let name = rule.name();

        context
            .report_diagnostic(
                RuleCodesInSelectors {
                    selector: selector.option,
                    in_lint_table: selector.in_lint_table,
                    name: name.as_str(),
                },
                selector.range,
            )
            .set_fix(Fix::safe_edit(Edit::range_replacement(
                name.to_string(),
                selector.range,
            )));
    }
}
