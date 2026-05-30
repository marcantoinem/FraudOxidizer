use std::str::FromStr;

use crate::ParseCsvError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MerchantCategory {
    Grocery,
    Gas,
    Restaurant,
    OnlineRetail,
    Electronics,
    Travel,
    Subscription,
    Entertainment,
    Utilities,
    Atm,
    GiftCard,
}

impl FromStr for MerchantCategory {
    type Err = ParseCsvError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let merchant_category = match value {
            "grocery" => MerchantCategory::Grocery,
            "gas" => MerchantCategory::Gas,
            "restaurant" => MerchantCategory::Restaurant,
            "online_retail" => MerchantCategory::OnlineRetail,
            "electronics" => MerchantCategory::Electronics,
            "travel" => MerchantCategory::Travel,
            "subscription" => MerchantCategory::Subscription,
            "entertainment" => MerchantCategory::Entertainment,
            "utilities" => MerchantCategory::Utilities,
            "atm" => MerchantCategory::Atm,
            "gift_card" => MerchantCategory::GiftCard,
            _ => {
                return Err(ParseCsvError::InvalidEnumValue {
                    field: "merchant_category",
                    value: value.to_string(),
                });
            }
        };

        Ok(merchant_category)
    }
}
