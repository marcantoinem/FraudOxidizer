use std::{fmt::Write as _, fs, path::Path};

use super::{
    optional_ip_addr, optional_string, parse_amount, parse_timestamp, transaction::Transaction,
};
use crate::{ParseCsvError, data::human_review_status::HumanReviewStatus};

#[derive(Debug, Default)]
pub struct Transactions {
    pub items: Vec<Transaction>,
}

impl Transactions {
    pub fn parse_csv<P: AsRef<Path>>(path: P) -> Result<Self, ParseCsvError> {
        let content = fs::read_to_string(path)?;
        Self::parse_csv_content(&content)
    }

    pub fn export_csv_content(&self) -> String {
        let mut output = String::new();
        output.push_str("transaction_id,timestamp,card_id,amount,merchant_name,merchant_category,channel,cardholder_country,merchant_country,device_id,ip_address,likely_fraud\n");

        for transaction in &self.items {
            let device_id = transaction.device_id.as_deref().unwrap_or("");
            let ip_address = transaction
                .ip_address
                .map(|ip_address| ip_address.to_string())
                .unwrap_or_default();

            writeln!(
                output,
                "tx_{:06},{},card_{:03},{},{},{},{},{},{},{},{},{}",
                transaction.transaction_id.0,
                transaction.timestamp.format("%Y-%m-%dT%H:%M:%S"),
                transaction.card_id.0,
                transaction.amount,
                transaction.merchant_name,
                serialize_merchant_category(transaction.merchant_category),
                serialize_channel(transaction.channel),
                transaction.cardholder_country.0.alpha2(),
                transaction.merchant_country.0.alpha2(),
                device_id,
                ip_address,
                transaction.likely_fraud_for_export(),
            )
            .expect("writing to a String cannot fail");
        }

        output
    }

    pub fn export_csv<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        fs::write(path, self.export_csv_content())
    }

    pub fn parse_csv_content(content: &str) -> Result<Self, ParseCsvError> {
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
                fraud_factors: Vec::new(),
                human_review_status: HumanReviewStatus::NotNeeded,
            });
        }

        Ok(Self { items })
    }
}

fn serialize_channel(channel: super::channel::Channel) -> &'static str {
    match channel {
        super::channel::Channel::Online => "online",
        super::channel::Channel::InPerson => "in_person",
        super::channel::Channel::Atm => "atm",
    }
}

fn serialize_merchant_category(
    category: super::merchant_category::MerchantCategory,
) -> &'static str {
    match category {
        super::merchant_category::MerchantCategory::Grocery => "grocery",
        super::merchant_category::MerchantCategory::Gas => "gas",
        super::merchant_category::MerchantCategory::Restaurant => "restaurant",
        super::merchant_category::MerchantCategory::OnlineRetail => "online_retail",
        super::merchant_category::MerchantCategory::Electronics => "electronics",
        super::merchant_category::MerchantCategory::Travel => "travel",
        super::merchant_category::MerchantCategory::Subscription => "subscription",
        super::merchant_category::MerchantCategory::Entertainment => "entertainment",
        super::merchant_category::MerchantCategory::Utilities => "utilities",
        super::merchant_category::MerchantCategory::Atm => "atm",
        super::merchant_category::MerchantCategory::GiftCard => "gift_card",
    }
}
