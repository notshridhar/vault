use std::collections::HashMap;
use std::iter;
use std::vec::Vec;

pub struct ParsedArgs {
    options: HashMap<String, String>,
}

impl ParsedArgs {
    pub fn from_args(arg_list: &[String]) -> Self {
        let mut options = HashMap::new();
        let mut arg_key = "0".to_owned();
        let mut arg_values = [arg_list[0].to_owned()].to_vec();
        let end_value = "--".to_owned();
        for arg in arg_list.iter().skip(1).chain(iter::once(&end_value)) {
            if arg.starts_with("--") && arg.contains('=') {
                // manage optional key with equal sign
                // example: app val1 [--opt2=val2]
                options.insert(arg_key, arg_values.join(" "));
                let mut split = arg.trim_start_matches('-').split('=');
                arg_key = split.next().unwrap().to_owned();
                arg_values = split.map(|x| x.to_owned()).collect();
            } else if arg.starts_with("--") {
                // manage optional key without equal sign
                // example: app val1 [--opt2] val2
                options.insert(arg_key, arg_values.join(" "));
                arg_key = arg.trim_start_matches('-').to_owned();
                arg_values = Vec::with_capacity(1);
            } else if let Ok(arg_index) = arg_key.parse::<u16>() {
                // manage required value
                // example: app [val1] --opt2 val2
                options.insert(arg_key, arg_values.join(" "));
                arg_key = (arg_index + 1).to_string();
                arg_values = Vec::with_capacity(1);
                arg_values.push(arg.to_owned());
            } else {
                // manage optional value without equal sign
                // example: app val1 --opt2 [val2]
                arg_values.push(arg.to_owned());
            }
        }
        Self { options }
    }

    pub fn get_index(&self, index: u16) -> Option<&str> {
        self.options.get(&index.to_string()).map(|x| x.as_str())
    }

    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.options.get(key).map(|x| x.as_str())
    }

    pub fn expect_index(
        &self, index: u16, key: &str
    ) -> Result<&str, ParserError> {
        self.options.get(&index.to_string())
            .map(|x| x.as_str())
            .ok_or(ParserError::missing_value(key))
    }

    pub fn expect_no_unrecognized(
        &self, unrecognized_index: u16, recognized_keys: &[&str]
    ) -> Result<(), ParserError> {
        if self.options.contains_key(&unrecognized_index.to_string()) {
            Err(ParserError::TooManyIndexed)
        } else {
            self.options
                .keys()
                .filter(|key| key.parse::<u16>().is_err()
                    && !recognized_keys.contains(&key.as_str()))
                .map(|key| ParserError::invalid_key(key))
                .next()
                .map_or(Ok(()), |err| Err(err))
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParserError {
    TooManyIndexed,
    InvalidKey { key: String },
    MissingValue { key: String },
    InvalidValue { key: String },
}

impl ParserError {
    pub fn invalid_key<S: Into<String>>(key: S) -> Self {
        Self::InvalidKey { key: key.into() }
    }

    pub fn missing_value<S: Into<String>>(key: S) -> Self {
        Self::MissingValue { key: key.into() }
    }

    pub fn invalid_value<S: Into<String>>(key: S) -> Self {
        Self::InvalidValue { key: key.into() }
    }
}

#[cfg(test)]
mod test {
    fn get_args_from_command(command: &str) -> Vec<String> {
        command.split(' ')
            .map(|x| x.to_owned())
            .collect::<Vec<_>>()
    }

    #[test]
    fn should_get_index_and_value() {
        let args = super::ParsedArgs {
            options: super::HashMap::from([
                ("0".to_owned(), "vlt".to_owned()),
                ("1".to_owned(), "set".to_owned()),
                ("key".to_owned(), "val".to_owned()),
            ])
        };
        assert_eq!(args.get_index(0), Some("vlt"));
        assert_eq!(args.get_index(1), Some("set"));
        assert_eq!(args.get_index(2), None);
        assert_eq!(args.get_value("key"), Some("val"));
        assert_eq!(args.get_value("val"), None);
        let error = super::ParserError::missing_value("key");
        assert_eq!(args.expect_index(2, "key"), Err(error));
        
    }

    #[test]
    fn should_parse_arg_list_indexed_only() {
        let args_list = get_args_from_command("vlt get sec/path");
        let args = super::ParsedArgs::from_args(&args_list);
        assert_eq!(args.get_index(0), Some("vlt"));
        assert_eq!(args.get_index(1), Some("get"));
        assert_eq!(args.get_index(2), Some("sec/path"));
        assert_eq!(args.get_index(3), None);
    }

    #[test]
    fn should_parse_arg_list_with_named() {
        let command = "vlt get --force --key val --key1 a b --key2=val";
        let args_list = get_args_from_command(command);
        let args = super::ParsedArgs::from_args(&args_list);
        assert_eq!(args.get_index(0), Some("vlt"));
        assert_eq!(args.get_index(1), Some("get"));
        assert_eq!(args.get_index(2), None);
        assert_eq!(args.get_value("force"), Some(""));
        assert_eq!(args.get_value("key"), Some("val"));
        assert_eq!(args.get_value("key1"), Some("a b"));
        assert_eq!(args.get_value("key2"), Some("val"));
        assert_eq!(args.get_value("forc"), None);
    }
}
