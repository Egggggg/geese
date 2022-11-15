use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Hex {
    pub inner: String,
}

#[derive(Clone, Debug)]
pub enum HexError {
    Validation(String),
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(inner) => {
                write!(f, "{inner} is an invalid CSS hex color code")
            }
        }
    }
}

impl Hex {
    fn new(input: impl ToString) -> Result<Self, HexError> {
        let input = input.to_string();

        if Self::validate(&input) {
            Ok(Self { inner: input })
        } else {
            Err(HexError::Validation(input))
        }
    }

    fn validate(input: impl ToString) -> bool {
        let input = &input.to_string();

        let re = Regex::new(r"#[\d{3}\d{6}]").unwrap();

        if re.is_match(input) {
            true
        } else {
            false
        }
    }

    fn set(&mut self, input: impl ToString) -> Result<(), HexError> {
        let input = input.to_string();

        if Self::validate(&input) {
            self.inner = input;
            Ok(())
        } else {
            Err(HexError::Validation(input))
        }
    }

    fn get(&self) -> &str {
        &self.inner
    }
}
