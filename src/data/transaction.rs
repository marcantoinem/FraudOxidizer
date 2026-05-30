use super::{
    card_id::CardId, channel::Channel, country::Country, merchant_category::MerchantCategory,
    transaction_id::TransactionId,
};

#[derive(Debug)]
pub struct Transaction {
    pub transaction_id: TransactionId,
    pub timestamp: String,
    pub card_id: CardId,
    pub amount: f64,
    pub merchant_name: String,
    pub merchant_category: MerchantCategory,
    pub channel: Channel,
    pub cardholder_country: Country,
    pub merchant_country: Country,
    pub device_id: Option<String>,
    pub ip_address: Option<String>,
}
