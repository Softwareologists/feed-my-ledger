use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Account {
    parts: Vec<String>,
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Account {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Account::from_str(&s).map_err(DeError::custom)
    }
}

impl FromStr for Account {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            parts: if s.is_empty() {
                Vec::new()
            } else {
                s.split(':').map(|p| p.to_string()).collect()
            },
        })
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.parts.join(":"))
    }
}

impl Account {
    pub fn starts_with(&self, other: &Account) -> bool {
        if other.parts.len() > self.parts.len() {
            return false;
        }
        self.parts.iter().zip(&other.parts).all(|(a, b)| a == b)
    }
}
