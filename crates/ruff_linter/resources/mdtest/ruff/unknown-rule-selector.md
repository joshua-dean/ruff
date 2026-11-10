# `unknown-rule-selector` (`RUF203`)

```toml
[lint]
preview = true
select = ["unknown-rule-selector"]
```

## Unknown selectors

A selector that doesn't match any rule, prefix, linter, or category is reported wherever selectors
are accepted. Rule codes, rule names, prefixes, linters, and categories are all recognized.

`ruff.toml`:

```toml
fixable = ["F4O1"]  # snapshot: unknown-rule-selector

[lint]
select = ["F401", "unused-import", "F4", "F", "ALL", "correctness"]  # no diagnostic
ignore = ['unused-imports']  # snapshot: unknown-rule-selector

[lint.per-file-ignores]
"tests/*" = ["S101", "XYZ"]  # snapshot: unknown-rule-selector
```

```snapshot
error[RUF203]: Unknown rule selector `F4O1` in `fixable`
 --> src/ruff.toml:1:13
  |
1 | fixable = ["F4O1"]  # snapshot: unknown-rule-selector
  |             ^^^^


error[RUF203]: Unknown rule selector `unused-imports` in `lint.ignore`
 --> src/ruff.toml:5:12
  |
5 | ignore = ['unused-imports']  # snapshot: unknown-rule-selector
  |            ^^^^^^^^^^^^^^


error[RUF203]: Unknown rule selector `XYZ` in `lint.per-file-ignores`
 --> src/ruff.toml:8:23
  |
8 | "tests/*" = ["S101", "XYZ"]  # snapshot: unknown-rule-selector
  |                       ^^^
```

## Removed selectors

The `NURSERY` and `PREVIEW` selectors were removed in favor of preview mode.

`ruff.toml`:

```toml
[lint]
extend-select = ["NURSERY"]  # snapshot: unknown-rule-selector
```

```snapshot
error[RUF203]: Removed rule selector `NURSERY` in `lint.extend-select`
 --> src/ruff.toml:2:19
  |
2 | extend-select = ["NURSERY"]  # snapshot: unknown-rule-selector
  |                   ^^^^^^^
```

## Redirected selectors

A selector that redirects to an existing rule or prefix is known. `renamed-rule` (`RUF202`) reports
it instead.

`ruff.toml`:

```toml
[lint]
select = ["TCH", "TCH002"]  # no diagnostic
```

## Selectors containing quotes

Only the quotes that delimit the TOML string are excluded from the reported range.

`ruff.toml`:

```toml
[lint]
select = ["'F401'"]  # snapshot: unknown-rule-selector
ignore = ["""F4O1"""]  # snapshot: unknown-rule-selector
```

```snapshot
error[RUF203]: Unknown rule selector `'F401'` in `lint.select`
 --> src/ruff.toml:2:12
  |
2 | select = ["'F401'"]  # snapshot: unknown-rule-selector
  |            ^^^^^^


error[RUF203]: Unknown rule selector `F4O1` in `lint.ignore`
 --> src/ruff.toml:3:14
  |
3 | ignore = ["""F4O1"""]  # snapshot: unknown-rule-selector
  |              ^^^^
```

## Other tables

Selectors are only checked in Ruff's own settings.

`pyproject.toml`:

```toml
[tool.other]
select = ["XYZ"]  # no diagnostic

[tool.ruff.lint]
select = ["XYZ"]  # snapshot: unknown-rule-selector
```

```snapshot
error[RUF203]: Unknown rule selector `XYZ` in `lint.select`
 --> src/pyproject.toml:5:12
  |
5 | select = ["XYZ"]  # snapshot: unknown-rule-selector
  |            ^^^
```
