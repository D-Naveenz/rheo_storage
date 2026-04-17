# AI Commit Message Prompt

Generate a git commit message from a git diff or staged diff.

Output the commit message only. Do not add commentary, labels, code fences,
or surrounding markup.

## Goal

Write a commit message that reflects the real center of gravity of the diff.
Do not just follow file order or always prefer a fixed type order. Use judgment
to decide what changed most meaningfully, then use the default type order only
as a tie-breaker when the diff is genuinely mixed.

## Output Format

Use this structure:

```text
<Overall Title>

<emoji> <type>【<scope>】: <Section Title>
- <bullet>
- <bullet>

<emoji> <type>【<scope>】: <Section Title>
- <bullet>
```

Rules:

- The first line is one strong overall title for the whole change.
- The body is grouped into typed sections.
- Each typed section may contain one or more bullets.
- Omit empty sections.
- Keep the whole message in English.
- Use lightweight Markdown only.
- Never wrap the output in triple backticks.

## Priority Rules

Choose the overall title from the most important change in the diff.

Judge importance using the actual impact of the change, such as:

- new capability or removed capability
- bug severity or correctness impact
- architectural or workflow impact
- user-facing effect
- how central the change is to the diff

Default type order when importance is otherwise close:

`feat > fix > docs > style > refactor > perf > test > build > ci > chore > i18n`

This order is a fallback, not a hard override. For example, a docs-heavy diff
may deserve a docs-first title even if it contains some incidental refactoring.

## Type Reference

| Type     | Emoji | Use for                                  |
| -------- | ----- | ---------------------------------------- |
| feat     | ✨    | New features or expanded capabilities    |
| fix      | 🐛    | Bug fixes or correctness repairs         |
| docs     | 📝    | Documentation changes                    |
| style    | 💄    | Pure presentation or formatting changes  |
| refactor | ♻️    | Structural cleanup without behavior gain |
| perf     | ⚡️    | Performance improvements                 |
| test     | ✅    | Test additions or test-only updates      |
| build    | 📦    | Build tooling or dependency changes      |
| ci       | 👷    | CI workflow changes                      |
| chore    | 🔧    | Maintenance work not better typed above  |
| i18n     | 🌐    | Localization changes                     |

## Writing Rules

### Overall Title

- Make it a concise, powerful summary of the whole change.
- Use normal title or sentence casing.
- Keep it specific and meaningful.
- Do not prefix it with a type or emoji.

### Section Headers

- Format each section as `emoji type【scope】: Title`.
- Keep `type` and `scope` in English.
- Use a short scope that names the affected area when it helps clarity.
- Omit unnecessary scope detail.
- Use normal title or sentence casing for the section title.

### Bullets

- Write plain bullet sentences.
- Do not repeat labels like `【docs】` inside bullets.
- Say what changed.
- Include why only when the reason is visible from the diff.
- Keep bullets concise and information-dense.

## Hard Constraints

- Output only the commit message.
- Do not explain your reasoning.
- Do not echo the input diff.
- Do not invent changes that are not supported by the diff.
- Do not create duplicate sections for the same type and scope unless the diff
  clearly contains separate themes that read better apart.

## Examples

### Example 1: Single-type refactor

Input: a diff that renames a server constant, moves startup parsing into a
helper, and does not change behavior.

Output:

Refine server startup structure

♻️ refactor【server】: Simplify startup configuration
- Rename the port constant to better match its role.
- Move startup parsing into a dedicated helper for clearer entry logic.

### Example 2: Feature-dominant mixed diff

Input: a diff that adds a new package inspection command, also cleans up old
tool modules, and updates the README.

Output:

Add package inspection support to the tooling flow

✨ feat【tooling】: Add package inspection command
- Register a new command path for inspecting packaged outputs.
- Expose package entry inspection from the tool runtime.

♻️ refactor【tooling】: Consolidate internal command modules
- Move shared command and shell helpers into the main tool crate.
- Remove split tool crates that no longer carry unique behavior.

📝 docs【README】: Refresh tooling crate descriptions
- Update the crate list to match the merged tool architecture.

### Example 3: Docs-dominant mixed diff

Input: a diff that rewrites a contributor guide, adds clearer prompt
instructions, and includes a few small code cleanups to match the new guide.

Output:

Rewrite the contributor prompt guide for AI workflows

📝 docs【ai】: Clarify commit message generation guidance
- Replace line-by-line prompt scripting with outcome-focused instructions.
- Explain how to choose the dominant change instead of following file order.

♻️ refactor【docs】: Align small supporting examples
- Clean up example fragments so they match the rewritten guidance.
