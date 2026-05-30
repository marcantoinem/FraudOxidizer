use std::str::FromStr;

use crate::ParseCsvError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Online,
    InPerson,
    Atm,
}

impl FromStr for Channel {
    type Err = ParseCsvError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let channel = match value {
            "online" => Channel::Online,
            "in_person" => Channel::InPerson,
            "atm" => Channel::Atm,
            _ => {
                return Err(ParseCsvError::InvalidEnumValue {
                    field: "channel",
                    value: value.to_string(),
                });
            }
        };

        Ok(channel)
    }
}
