use std::collections::HashMap;

pub trait Serialize {
    fn serialize(self) -> String;
}

impl Serialize for &String {
    #[inline]
    fn serialize(self) -> String {
        self.to_owned()
    }
}

impl Serialize for &str {
    #[inline]
    fn serialize(self) -> String {
        self.to_owned()
    }
}

impl Serialize for &HashMap<String, u32> {
    fn serialize(self) -> String {
        let approx_size = self.len() * 10;
        let mut result = String::with_capacity(approx_size);
        result.push('{');
        self.into_iter().for_each(|(key, value)| {
            result.push('"');
            result.push_str(&key);
            result.push('"');
            result.push(':');
            result.push_str(&value.to_string());
            result.push(',');
        });
        if result.len() > 1 {
            result.pop();
        }
        result.push('}');
        result
    }
}

pub trait Deserialize {
    fn deserialize(val: &str) -> Option<Self>
    where Self: Sized;
}

impl Deserialize for String {
    #[inline]
    fn deserialize(val: &str) -> Option<Self> {
        Some(val.to_owned())
    }
}

impl Deserialize for HashMap<String, u32> {
    fn deserialize(val: &str) -> Option<Self> {
        if val.starts_with('{') && val.ends_with('}') {
            let result = val[1..val.len() - 1]
                .split(',')
                .filter_map(|line|
                    line.split_once(':').map(|(key, val)| (
                        key.trim().trim_matches('"').to_owned(),
                        val.trim().parse::<u32>().unwrap()
                    ))
                )
                .collect::<HashMap<_, _>>();
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use super::{Serialize, Deserialize};

    #[test]
    fn should_serialize_string_as_is() {
        let value = "1234";
        assert_eq!(value.serialize(), value)
    }

    #[test]
    fn should_deserialize_string_as_is() {
        let value = "1234".to_owned();
        assert_eq!(String::deserialize(&value), Some(value))
    }

    #[test]
    fn should_serialize_empty_hashmap() {
        let value = HashMap::new();
        assert_eq!(value.serialize(), "{}")
    }

    #[test]
    fn should_deserialize_empty_hashmap() {
        let value = HashMap::new();
        assert_eq!(HashMap::deserialize("{}"), Some(value))
    }

    #[test]
    fn should_serialize_non_empty_hashmap() {
        let value = HashMap::from([
            ("key1".to_owned(), 123),
            ("key2".to_owned(), 321)
        ]);
        let option1 = "{\"key1\":123,\"key2\":321}";
        let option2 = "{\"key2\":321,\"key1\":123}";
        assert!([option1, option2].contains(&value.serialize().as_str()))
    }

    #[test]
    fn should_deserialize_non_empty_hashmap() {
        let value = HashMap::from([
            ("key1".to_owned(), 123),
            ("key2".to_owned(), 321)
        ]);
        let serialized = "{\"key1\":123,\"key2\":321}";
        assert_eq!(HashMap::deserialize(serialized), Some(value));
    }

    #[test]
    fn should_not_deserialize_invalid_hashmap() {
        let serialized = "{\"key1\":123,\"key2\":321";
        assert_eq!(HashMap::deserialize(serialized), None);
        let serialized = "{\"key1\":123,\"key2\"321";
        assert_eq!(HashMap::deserialize(serialized), None);
    }
}
