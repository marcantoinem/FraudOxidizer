use std::{fs, path::Path};

use super::{
    optional_ip_addr, optional_string, parse_amount, parse_timestamp, transaction::Transaction,
};
use crate::ParseCsvError;

#[derive(Debug, Default)]
pub struct Transactions {
    pub items: Vec<Transaction>,
}

impl Transactions {
    pub fn parse_csv<P: AsRef<Path>>(path: P) -> Result<Self, ParseCsvError> {
        let content = fs::read_to_string(path)?;
        let mut lines = content.lines();

        let _header = lines.next().ok_or(ParseCsvError::MissingHeader)?;
        let mut items = Vec::new();

        for (line_index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let columns: Vec<&str> = line.split(',').collect();

            if columns.len() != 11 {
                return Err(ParseCsvError::InvalidColumnCount {
                    expected: 11,
                    found: columns.len(),
                    line: line_index + 2,
                });
            }

            items.push(Transaction {
                transaction_id: columns[0].parse()?,
                timestamp: parse_timestamp(columns[1])?,
                card_id: columns[2].parse()?,
                amount: parse_amount(columns[3])?,
                merchant_name: columns[4].to_string(),
                merchant_category: columns[5].parse()?,
                channel: columns[6].parse()?,
                cardholder_country: columns[7].parse()?,
                merchant_country: columns[8].parse()?,
                device_id: optional_string(columns[9]),
                ip_address: optional_ip_addr(columns[10])?,
            });
        }

        Ok(Self { items })
    }
}
