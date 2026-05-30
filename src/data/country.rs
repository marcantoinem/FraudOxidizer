use std::str::FromStr;

use crate::ParseCsvError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    Ca,
    Us,
    Fr,
    Gb,
    De,
    Cn,
    Se,
    Mx,
}

impl FromStr for Country {
    type Err = ParseCsvError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let country = match value {
            "CA" => Country::Ca,
            "US" => Country::Us,
            "FR" => Country::Fr,
            "GB" => Country::Gb,
            "DE" => Country::De,
            "CN" => Country::Cn,
            "SE" => Country::Se,
            "MX" => Country::Mx,
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
