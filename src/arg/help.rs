use std::cmp;

const SECTION_DEF: &str = "__section__";

/// Generate formatted help string from given specs.
pub struct HelpGenerator {
    specs: Vec<(String, String)>,
}

impl HelpGenerator {
    /// Constructs new generator.
    pub fn new() -> Self {
        Self { specs: Vec::with_capacity(20) }
    }

    /// Ends the last section, and adds a new section.
    /// Pushing further lines append them to this section.
    pub fn push_section(&mut self, name: &str) {
        self.specs.push((SECTION_DEF.to_owned(), name.to_owned()))
    }

    /// Pushes the given help key and value pairs.
    pub fn push_line(&mut self, key: &str, value: &str) {
        value
            .trim()
            .split('\n')
            .enumerate()
            .for_each(|(i, line)| {
                let line_trim = line.trim().to_owned();
                let key = if i == 0 { key.to_owned() } else { String::new() };
                self.specs.push((key, line_trim))
            });
    }

    /// Generates formatted help string.
    pub fn generate(self) -> String {
        let space = self.specs
            .iter()
            .fold(0, |max, item| cmp::max(item.0.len(), max));
        self.specs
            .iter()
            .enumerate()
            .fold(String::with_capacity(200), |mut result, (i, spec)| {
                let (name, value) = spec;
                if i > 0 {
                    result.push('\n');
                }
                if name == &SECTION_DEF {
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
                let next_name = self.specs
                    .get(i + 1)
                    .map_or(String::new(), |item| item.0.clone());
                if next_name == SECTION_DEF {
                    result.push('\n');
                }
                result
            })
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn should_generate_simple_help_string() {
        let mut generator = super::HelpGenerator::new();
        generator.push_section("usage");
        generator.push_line("", "vault [options]");
        let help_string = generator.generate();
        assert_eq!(help_string, "usage:\n    vault [options]")
    }

    #[test]
    fn should_generate_multiline_spec_help_string() {
        let mut generator = super::HelpGenerator::new();
        generator.push_section("usage");
        generator.push_line("", "
            a
            b
        ");
        let help_string = generator.generate();
        assert_eq!(help_string, "usage:\n    a\n    b")
    }
}
