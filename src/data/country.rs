use std::str::FromStr;

use my_country::Country as CountryCode;

use crate::ParseCsvError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Country(pub CountryCode);

impl FromStr for Country {
    type Err = ParseCsvError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        CountryCode::from_str(value)
            .map(Self)
            .map_err(|_| ParseCsvError::InvalidEnumValue {
                field: "country",
                value: value.to_string(),
            })
    }
}
