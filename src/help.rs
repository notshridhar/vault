use std::cmp;

/// Generates formatted help string from given specs.
pub fn get_help_string(specs: &[(&str, &str)]) -> String {
    let space = specs
        .iter()
        .filter(|(name, _)| name != &"_section")
        .fold(0, |max, (name, _)| cmp::max(name.len(), max));

    specs
        .iter()
        .enumerate()
        .fold(String::with_capacity(200), |mut result, (i, (name, value))| {
            if i > 0 {
                result.push('\n');
            }

            if name == &"_section" {
                result.push_str(value);
                result.push(':');
            } else if name.is_empty() {
                result.push_str(&" ".repeat(4));
                result.push_str(value);
            } else {
                result.push_str(&" ".repeat(4));
                result.push_str(name);
                result.push_str(&" ".repeat(space - name.len() + 2));
                result.push_str(value);
            }

            let next_name = specs.get(i + 1).map_or("", |item| item.0);
            if next_name == "_section" {
                result.push('\n');
            }

            result
        })
}

#[cfg(test)]
mod test {
    #[test]
    fn should_get_help_string_as_expected() {
        let help_string = super::get_help_string(&[
            ("_section", "usage"),
            ("", "vault [options]"),
        ]);
        assert_eq!(help_string, "usage:\n    vault [options]")
    }
}
