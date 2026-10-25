use anyhow::Result;
use insta::assert_json_snapshot;
use lsp_types::{Position, Range};

use crate::TestServerBuilder;

#[test]
fn no_hover_for_markdown() -> Result<()> {
    let mut server = TestServerBuilder::new()?.with_workspace(".")?.build();

    server.open_text_document_with_language_id("test.md", "markdown", "# noqa: RUF100\n", 1);

    let result = server.hover_request(
        "test.md",
        Position {
            line: 0,
            character: 9,
        },
    );

    assert!(result.is_none(), "Expected no hover for markdown file");

    Ok(())
}

#[test]
fn hover_for_python_noqa() -> Result<()> {
    let mut server = TestServerBuilder::new()?.with_workspace(".")?.build();

    server.open_text_document("test.py", "x = 1  # noqa: RUF100\n", 1);

    let result = server.hover_request(
        "test.py",
        Position {
            line: 0,
            character: 16,
        },
    );

    assert_json_snapshot!(
        result,
        @r##"
    {
      "contents": {
        "kind": "markdown",
        "value": "# unused-noqa (RUF100)\n\nDerived from the **Ruff-specific rules** linter.\n\nFix is always available.\n\n## What it does\nChecks for `noqa` directives that are no longer applicable.\n\n## Why is this bad?\nA `noqa` directive that no longer matches any diagnostic violations is\nlikely included by mistake, and should be removed to avoid confusion.\n\n## Example\n```python\nimport foo  # noqa: F401\n\n\ndef bar():\n    foo.bar()\n```\n\nUse instead:\n```python\nimport foo\n\n\ndef bar():\n    foo.bar()\n```\n\n## Conflict with other linters\nWhen using `RUF100` with the `--fix` option, Ruff may remove trailing comments\nthat follow a `# noqa` directive on the same line, as it interprets the\nremainder of the line as a description for the suppression.\n\nTo prevent Ruff from removing suppressions for other tools (like `pylint`\nor `mypy`), separate them with a second `#` character:\n\n```python\n# Bad: Ruff --fix will remove the pylint comment\ndef visit_ImportFrom(self, node):  # noqa: N802, pylint: disable=invalid-name\n    pass\n\n\n# Good: Ruff will preserve the pylint comment\ndef visit_ImportFrom(self, node):  # noqa: N802 # pylint: disable=invalid-name\n    pass\n```\n\n## Fix safety\n\nThe rule's fix is marked as unsafe when a full suppression comment would be removed and there\nare other nested comments on the same line. Removing such a comment can change the behavior of\nother suppression comments before or after the removed comment.\n\n## See also\n\nThis rule ignores any codes that are unknown to Ruff, as it can't determine\nif the codes are valid or used by other tools. Enable [`invalid-rule-code`][RUF102]\nto flag any unknown rule codes.\n\n## References\n- [Ruff error suppression](https://docs.astral.sh/ruff/linter/#error-suppression)\n\n[RUF102]: https://docs.astral.sh/ruff/rules/invalid-rule-code/"
      },
      "range": {
        "start": {
          "line": 0,
          "character": 15
        },
        "end": {
          "line": 0,
          "character": 21
        }
      }
    }
    "##
    );

    Ok(())
}

#[test]
fn hover_for_file_level_noqa() -> Result<()> {
    let mut server = TestServerBuilder::new()?.with_workspace(".")?.build();

    server.open_text_document("test.py", "# ruff: noqa: F401\n", 1);

    let result = server
        .hover_request(
            "test.py",
            Position {
                line: 0,
                character: 15,
            },
        )
        .expect("Expected hover information for a file-level `noqa` directive");

    let lsp_types::Contents::MarkupContent(markup) = result.contents else {
        panic!("Expected Markdown hover contents");
    };
    assert!(markup.value.starts_with("# unused-import (F401)"));

    Ok(())
}

#[test]
fn hover_for_ruff_suppression_comments() -> Result<()> {
    let mut server = TestServerBuilder::new()?.with_workspace(".")?.build();

    server.open_text_document(
        "test.py",
        "# ruff: disable[F401]\n# ruff: enable[F401]\n# ruff: file-ignore[F401]\n# ruff: ignore[F401]\n# ruff: disable[F401] # ruff: ignore[F401]\n# ruff: ignore[F401,]\n",
        1,
    );

    for position in [
        Position {
            line: 0,
            character: 17,
        },
        Position {
            line: 1,
            character: 16,
        },
        Position {
            line: 2,
            character: 21,
        },
        Position {
            line: 3,
            character: 16,
        },
        Position {
            line: 4,
            character: 38,
        },
        Position {
            line: 5,
            character: 16,
        },
    ] {
        let result = server
            .hover_request("test.py", position)
            .expect("Expected hover information for a Ruff suppression comment");

        let lsp_types::Contents::MarkupContent(markup) = result.contents else {
            panic!("Expected Markdown hover contents");
        };
        assert!(markup.value.starts_with("# unused-import (F401)"));
    }

    Ok(())
}

#[test]
fn hover_for_human_readable_rule_name() -> Result<()> {
    let mut server = TestServerBuilder::new()?
        .with_workspace(".")?
        .with_file(
            "pyproject.toml",
            r#"
[tool.ruff]
preview = true
"#,
        )?
        .build();

    server.open_text_document("test.py", "éééé = 1  # ruff: ignore[unused-import]\n", 1);

    let result = server
        .hover_request(
            "test.py",
            Position {
                line: 0,
                character: 26,
            },
        )
        .expect("Expected hover information for a human-readable rule name");

    let lsp_types::Contents::MarkupContent(markup) = result.contents else {
        panic!("Expected Markdown hover contents");
    };
    assert!(markup.value.starts_with("# unused-import (F401)"));
    assert_eq!(
        result.range,
        Some(Range {
            start: Position {
                line: 0,
                character: 25,
            },
            end: Position {
                line: 0,
                character: 38,
            },
        })
    );

    Ok(())
}

#[test]
fn no_hover_for_suppression_text_in_string() -> Result<()> {
    let mut server = TestServerBuilder::new()?.with_workspace(".")?.build();

    server.open_text_document(
        "test.py",
        "\"\"\"\nnot a comment # ruff:ignore[unused-import]\n\"\"\"\n",
        1,
    );

    let result = server.hover_request(
        "test.py",
        Position {
            line: 1,
            character: 31,
        },
    );

    assert!(result.is_none());

    Ok(())
}

#[test]
fn unavailable_document_returns_empty_response() -> Result<()> {
    let mut server = TestServerBuilder::new()?.with_workspace(".")?.build();

    let result = server.hover_request("not-open.py", Position::default());

    assert!(result.is_none());

    Ok(())
}

/// Write a TOML configuration file, open it in the editor, and request hover information.
fn toml_hover(
    file_name: &str,
    contents: &str,
    position: Position,
) -> Result<Option<lsp_types::Hover>> {
    toml_hover_unsaved(file_name, contents, contents, position)
}

/// Like [`toml_hover`], but the editor contents differ from the file on disk.
///
/// Ruff loads its settings from the file on disk, which must be a valid configuration to avoid
/// an error notification. The editor contents can be invalid, as they often are while typing.
fn toml_hover_unsaved(
    file_name: &str,
    saved: &str,
    unsaved: &str,
    position: Position,
) -> Result<Option<lsp_types::Hover>> {
    let mut server = TestServerBuilder::new()?
        .with_workspace(".")?
        .with_file(file_name, saved)?
        .build();

    server.open_text_document_with_language_id(file_name, "toml", unsaved, 1);

    Ok(server.hover_request(file_name, position))
}

fn markdown(hover: lsp_types::Hover) -> String {
    let lsp_types::Contents::MarkupContent(markup) = hover.contents else {
        panic!("Expected Markdown hover contents");
    };
    markup.value
}

#[test]
fn hover_for_rule_code_in_pyproject() -> Result<()> {
    let result = toml_hover(
        "pyproject.toml",
        "[tool.ruff.lint]\nselect = [\"F401\"]\n",
        Position {
            line: 1,
            character: 12,
        },
    )?
    .expect("Expected hover information for a rule code");

    assert_eq!(
        result.range,
        Some(Range {
            start: Position {
                line: 1,
                character: 11,
            },
            end: Position {
                line: 1,
                character: 15,
            },
        })
    );
    assert!(markdown(result).starts_with("# unused-import (F401)"));

    Ok(())
}

#[test]
fn hover_for_prefix_in_ruff_toml() -> Result<()> {
    let result = toml_hover(
        "ruff.toml",
        "[lint]\nignore = [\"E4\"]\n",
        Position {
            line: 1,
            character: 12,
        },
    )?
    .expect("Expected hover information for a prefix selector");

    assert_json_snapshot!(result);

    Ok(())
}

#[test]
fn hover_for_per_file_ignores_value() -> Result<()> {
    let source = "[tool.ruff.lint.per-file-ignores]\n\"tests/**\" = [\"S101\"]\n";

    // The file pattern is a key, not a selector.
    let on_pattern = toml_hover(
        "pyproject.toml",
        source,
        Position {
            line: 1,
            character: 3,
        },
    )?;
    assert!(on_pattern.is_none());

    let on_selector = toml_hover(
        "pyproject.toml",
        source,
        Position {
            line: 1,
            character: 16,
        },
    )?
    .expect("Expected hover information for a per-file-ignores selector");
    assert!(markdown(on_selector).starts_with("# assert (S101)"));

    Ok(())
}

#[test]
fn hover_for_redirected_selectors() -> Result<()> {
    let rule = toml_hover(
        "ruff.toml",
        "[lint]\nselect = [\"SIM111\"]\n",
        Position {
            line: 1,
            character: 12,
        },
    )?
    .expect("Expected hover information for a redirected rule code");
    assert!(markdown(rule).ends_with("`SIM111` has been remapped to `SIM110`."));

    let prefix = toml_hover(
        "ruff.toml",
        "[lint]\nselect = [\"C9\"]\n",
        Position {
            line: 1,
            character: 12,
        },
    )?
    .expect("Expected hover information for a redirected prefix");
    assert_json_snapshot!(prefix);

    Ok(())
}

#[test]
fn hover_for_rule_name_requires_preview() -> Result<()> {
    let position = Position {
        line: 2,
        character: 12,
    };

    let stable = toml_hover_unsaved(
        "ruff.toml",
        "[lint]\npreview = false\nselect = [\"F401\"]\n",
        "[lint]\npreview = false\nselect = [\"unused-import\"]\n",
        position,
    )?;
    assert!(stable.is_none());

    let preview = toml_hover(
        "ruff.toml",
        "[lint]\npreview = true\nselect = [\"unused-import\"]\n",
        position,
    )?
    .expect("Expected hover information for a rule name in preview");
    assert!(markdown(preview).starts_with("# unused-import (F401)"));

    Ok(())
}

#[test]
fn no_hover_outside_ruff_selectors() -> Result<()> {
    let source = "[tool.other]\nselect = [\"F401\"]\n\n[tool.ruff.format]\nselect = [\"F401\"]\n\n[tool.ruff.lint]\nselect = [\"F401\", \"NOT-A-RULE\"]\n";
    // `select` isn't a valid `format` option and `NOT-A-RULE` isn't a valid selector, so only
    // the editor contents contain them.
    let saved = "[tool.ruff.lint]\nselect = [\"F401\"]\n";

    for position in [
        // `[tool.other]`
        Position {
            line: 1,
            character: 12,
        },
        // `[tool.ruff.format]`
        Position {
            line: 4,
            character: 12,
        },
        // The `select` key itself
        Position {
            line: 7,
            character: 2,
        },
        // An unknown selector
        Position {
            line: 7,
            character: 22,
        },
    ] {
        let result = toml_hover_unsaved("pyproject.toml", saved, source, position)?;
        assert!(result.is_none(), "Expected no hover at {position:?}");
    }

    Ok(())
}

#[test]
fn no_hover_for_invalid_toml() -> Result<()> {
    // The unsaved editor contents are missing the closing bracket.
    let result = toml_hover_unsaved(
        "ruff.toml",
        "[lint]\nselect = [\"F401\"]\n",
        "[lint]\nselect = [\"F401\"\n",
        Position {
            line: 1,
            character: 12,
        },
    )?;

    assert!(result.is_none());

    Ok(())
}
