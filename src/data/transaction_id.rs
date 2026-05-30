use std::str::FromStr;

use super::parse_prefixed_number;
use crate::ParseCsvError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionId(pub u64);

impl FromStr for TransactionId {
    type Err = ParseCsvError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_prefixed_number(value, "tx_").map(Self)
    }
}
