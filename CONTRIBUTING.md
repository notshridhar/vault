# Contributing guide

Before committing, follow these steps:
- `cargo test` - this should pass all the test cases.
- `cargo clippy -- -A clippy::option_map_unit_fn` - this should not produce any warnings or errors.
- `'.{80}'` in vscode regex search should not find any.
- `' $'` in vscode regex search should not find any.
