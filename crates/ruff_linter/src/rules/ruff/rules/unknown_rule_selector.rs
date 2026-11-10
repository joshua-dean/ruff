use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_text_size::TextRange;

use crate::Violation;
use crate::checkers::ast::LintContext;
use crate::codes::Category;
use crate::rule_selector::RuleResolutionErrorKind;

/// ## What it does
///
/// Checks Ruff configuration files for rule selectors that Ruff doesn't recognize.
///
/// ## Why is this bad?
///
/// A selector that doesn't match any rule has no effect. It's usually a typo or the code of a
/// rule that has since been removed, so the rules that it was meant to enable or ignore are
/// silently left unchanged.
///
/// ## Example
///
/// ```toml
/// [tool.ruff.lint]
/// select = ["F4O1"]
/// ```
///
/// Use instead:
///
/// ```toml
/// [tool.ruff.lint]
/// select = ["F401"]
/// ```
#[derive(ViolationMetadata)]
#[violation_metadata(preview_since = "NEXT_RUFF_VERSION", category = Category::Correctness)]
pub(crate) struct UnknownRuleSelector {
    selector: String,
    kind: RuleResolutionErrorKind,
    setting: &'static str,
    in_lint_table: bool,
}

impl Violation for UnknownRuleSelector {
    #[derive_message_formats]
    fn message(&self) -> String {
        let Self {
            selector,
            kind,
            setting,
            in_lint_table,
        } = self;
        let table = if *in_lint_table { "lint." } else { "" };
        match kind {
            RuleResolutionErrorKind::Unknown => {
                format!("Unknown rule selector `{selector}` in `{table}{setting}`")
            }
            RuleResolutionErrorKind::Removed => {
                format!("Removed rule selector `{selector}` in `{table}{setting}`")
            }
            RuleResolutionErrorKind::PreviewCategory => {
                format!(
                    "Rule selector `{selector}` in `{table}{setting}` selects rules by category, \
                    which requires preview mode"
                )
            }
            RuleResolutionErrorKind::PreviewName => {
                format!(
                    "Rule selector `{selector}` in `{table}{setting}` selects a rule by name, \
                    which requires preview mode"
                )
            }
        }
    }
}

/// `RUF203`
pub(crate) fn unknown_rule_selector(
    context: &LintContext,
    selector: &str,
    kind: RuleResolutionErrorKind,
    range: TextRange,
    setting: &'static str,
    in_lint_table: bool,
) {
    context.report_diagnostic(
        UnknownRuleSelector {
            selector: selector.to_string(),
            kind,
            setting,
            in_lint_table,
        },
        range,
    );
}
