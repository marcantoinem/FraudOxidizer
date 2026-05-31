use super::{
    card_id::CardId, channel::Channel, country::Country, merchant_category::MerchantCategory,
    transaction_id::TransactionId,
};
use crate::{data::human_review_status::HumanReviewStatus, process::card_statistics::FraudFactor};

use chrono::{DateTime, Utc};
use std::net::IpAddr;

#[derive(Debug)]
pub struct Transaction {
    pub transaction_id: TransactionId,
    pub timestamp: DateTime<Utc>,
    pub card_id: CardId,
    pub amount: f64,
    pub merchant_name: String,
    pub merchant_category: MerchantCategory,
    pub channel: Channel,
    pub cardholder_country: Country,
    pub merchant_country: Country,
    pub device_id: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub fraud_factors: Vec<FraudFactor>,
    pub human_review_status: HumanReviewStatus,
}

impl Transaction {
    pub fn likely_fraud_for_export(&self) -> bool {
        self.human_review_status
            .likely_fraud_override()
            .unwrap_or_else(|| {
                self.fraud_factors
                    .iter()
                    .map(FraudFactor::weight)
                    .sum::<f32>()
                    >= crate::process::card_statistics::FRAUD_SCORE_THRESHOLD
            })
    }
}
