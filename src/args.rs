use std::collections::HashMap;

type ParserResult<T> = Result<T, ParserError>;

pub struct ParsedArgs {
    options: HashMap<String, String>,
}

impl ParsedArgs {
    pub fn from_args(arg_list: &[String]) -> Self {
        let mut options = HashMap::new();
        let mut arg_key = "-1".to_owned();
        for arg in arg_list.iter() {
            if arg.starts_with("--") && arg.contains('=') {
                // manage optional key with equal sign
                // example: app val1 [--opt2=val2]
                let (key, value) = arg.trim_start_matches('-')
                    .split_once('=').unwrap();
                options.insert(key.to_owned(), value.to_owned());
                arg_key = key.to_owned();
            } else if arg.starts_with("--") {
                // manage optional key without equal sign
                // example: app val1 [--opt2] val2
                let key = arg.trim_start_matches('-');
                options.insert(key.to_owned(), "".to_owned());
                arg_key = key.to_owned();
            } else if let Ok(arg_index) = arg_key.parse::<i16>() {
                // manage required value
                // example: app [val1] --opt2 val2
                let key = (arg_index + 1).to_string();
                options.insert(key.to_owned(), arg.to_owned());
                arg_key = key;
            } else {
                // manage optional value without equal sign
                // example: app val1 --opt2 [val2]
                let value = options.get_mut(&arg_key).unwrap();
                if !value.is_empty() { value.push_str(" ") }
                value.push_str(arg)
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

    pub fn expect_index(&self, index: u16, key: &str) -> ParserResult<&str> {
        self.get_index(index).ok_or(ParserError::missing_value(key))
    }

    pub fn expect_none_except(
        &self, index: u16, keys: &[&str]
    ) -> ParserResult<()> {
        if self.options.contains_key(&index.to_string()) {
            Err(ParserError::TooManyIndexed)
        } else {
            self.options
                .keys()
                .filter(|key|
                    key.parse::<u16>().is_err() &&
                    !keys.contains(&key.as_str())
                )
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

    #[test]
    fn should_throw_error_for_unrecognized_args() {
        let command = "vlt get --force --key val";
        let args_list = get_args_from_command(command);
        let args = super::ParsedArgs::from_args(&args_list);
        let error = Err(super::ParserError::TooManyIndexed);
        assert_eq!(args.expect_none_except(0, &[]), error);
        let error = Err(super::ParserError::invalid_key("force"));
        assert_eq!(args.expect_none_except(2, &["key"]), error);
        assert_eq!(args.expect_none_except(2, &["key", "force"]), Ok(()));
    }
}
