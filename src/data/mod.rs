pub mod card_id;
pub mod channel;
pub mod country;
pub mod merchant_category;
pub mod transaction;
pub mod transaction_id;
pub mod transactions;

use std::num::{ParseFloatError, ParseIntError};

use crate::ParseCsvError;

pub(crate) fn parse_prefixed_number(
    value: &str,
    prefix: &'static str,
) -> Result<u64, ParseCsvError> {
    let suffix = value
        .strip_prefix(prefix)
        .ok_or(ParseCsvError::InvalidPrefix {
            field: if prefix == "tx_" {
                "transaction_id"
            } else {
                "card_id"
            },
            expected_prefix: prefix,
            value: value.to_string(),
        })?;

    suffix
        .parse::<u64>()
        .map_err(|_: ParseIntError| ParseCsvError::InvalidNumber {
            field: if prefix == "tx_" {
                "transaction_id"
            } else {
                "card_id"
            },
            value: value.to_string(),
        })
}

pub(crate) fn parse_amount(value: &str) -> Result<f64, ParseCsvError> {
    value
        .parse::<f64>()
        .map_err(|_: ParseFloatError| ParseCsvError::InvalidFloat {
            field: "amount",
            value: value.to_string(),
        })
}

pub(crate) fn optional_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
