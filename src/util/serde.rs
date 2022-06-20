use std::collections::HashMap;

pub trait Serialize {
    fn serialize(self) -> String;
}

impl Serialize for &String {
    #[inline(always)]
    fn serialize(self) -> String {
        self.to_owned()
    }
}

impl Serialize for &str {
    #[inline(always)]
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
    #[inline(always)]
    fn deserialize(val: &str) -> Option<Self> {
        Some(val.to_owned())
    }
}

impl Deserialize for HashMap<String, u32> {
    fn deserialize(_val: &str) -> Option<Self> {
        Some(HashMap::new())
    }
}