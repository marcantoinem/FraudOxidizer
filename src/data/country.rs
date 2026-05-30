use std::str::FromStr;

use crate::ParseCsvError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    Canada,
    UnitedStates,
    France,
    UnitedKingdom,
    Germany,
    China,
    Sweden,
    Mexico,
}

impl FromStr for Country {
    type Err = ParseCsvError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let country = match value {
            "CA" => Country::Canada,
            "US" => Country::UnitedStates,
            "FR" => Country::France,
            "GB" => Country::UnitedKingdom,
            "DE" => Country::Germany,
            "CN" => Country::China,
            "SE" => Country::Sweden,
            "MX" => Country::Mexico,
            _ => {
                return Err(ParseCsvError::InvalidEnumValue {
                    field: "country",
                    value: value.to_string(),
                });
            }
        };

        Ok(country)
    }
}
