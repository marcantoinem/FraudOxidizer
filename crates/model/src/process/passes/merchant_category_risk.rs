use std::collections::BTreeMap;

use crate::data::transactions::Transactions;
use crate::process::card_statistics::{
    CASHOUT_MIN_AMOUNT, FraudFactor, category_price_deviation_weight, is_risky_category,
};

pub fn apply(transactions: &mut Transactions) {
    let mut by_card_amounts: BTreeMap<u64, Vec<f64>> = BTreeMap::new();

    for transaction in &transactions.items {
        by_card_amounts
            .entry(transaction.card_id.0)
            .or_default()
            .push(transaction.amount);
    }

    let mut baselines: BTreeMap<u64, (f64, f64)> = BTreeMap::new();
    for (card_id, amounts) in &mut by_card_amounts {
        amounts.sort_by(f64::total_cmp);
        let median_value = median_of_sorted(amounts);
        let mut abs_dev: Vec<f64> = amounts
            .iter()
            .map(|amount| (amount - median_value).abs())
            .collect();
        abs_dev.sort_by(f64::total_cmp);
        let mad = median_of_sorted(&abs_dev);
        baselines.insert(*card_id, (median_value, mad));
    }

    for transaction in &mut transactions.items {
        if !is_risky_category(transaction.merchant_category) {
            continue;
        }

        transaction
            .fraud_factors
            .push(FraudFactor::RiskyMerchantCategory {
                category: transaction.merchant_category,
            });

        if transaction.amount < CASHOUT_MIN_AMOUNT {
            continue;
        }

        let Some((card_median, card_mad)) = baselines.get(&transaction.card_id.0).copied() else {
            continue;
        };

        let robust_scale = (card_mad * 1.4826).max(1.0);
        if transaction.amount <= card_median {
            continue;
        }

        let z_score = (transaction.amount - card_median) / robust_scale;
        let weight = category_price_deviation_weight(z_score);
        if weight <= 0.0 {
            continue;
        }

        transaction
            .fraud_factors
            .push(FraudFactor::CategoryPriceDeviation {
                category: transaction.merchant_category,
                amount: transaction.amount,
                average_amount: card_median,
                std_deviation: robust_scale,
                z_score,
                weight,
            });
    }
}

fn median_of_sorted(sorted_values: &[f64]) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let mid = sorted_values.len() / 2;
    if sorted_values.len() % 2 == 1 {
        sorted_values[mid]
    } else {
        (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

    use crate::data::{
        card_id::CardId, channel::Channel, country::Country,
        human_review_status::HumanReviewStatus, merchant_category::MerchantCategory,
        transaction::Transaction, transaction_id::TransactionId,
    };
    use my_country::Country as CountryCode;

    fn tx(transaction_id: u64, amount: f64, category: MerchantCategory) -> Transaction {
        Transaction {
            transaction_id: TransactionId(transaction_id),
            timestamp: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date"),
                    NaiveTime::from_hms_opt(10, 0, 0).expect("valid time"),
                ),
                Utc,
            ),
            card_id: CardId(1),
            amount,
            merchant_name: "merchant".to_owned(),
            merchant_category: category,
            channel: Channel::Online,
            cardholder_country: Country(CountryCode::CA),
            merchant_country: Country(CountryCode::CA),
            device_id: None,
            ip_address: None,
            fraud_factors: Vec::new(),
            human_review_status: HumanReviewStatus::NotNeeded,
        }
    }

    #[test]
    fn adds_risky_category_factor() {
        let mut transactions = Transactions {
            items: vec![tx(1, 30.0, MerchantCategory::GiftCard)],
        };

        apply(&mut transactions);

        assert!(transactions.items[0].fraud_factors.iter().any(|factor| {
            matches!(
                factor,
                FraudFactor::RiskyMerchantCategory {
                    category: MerchantCategory::GiftCard
                }
            )
        }));
    }

    #[test]
    fn adds_deviation_factor_when_amount_is_well_above_category_average() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, 100.0, MerchantCategory::Electronics),
                tx(2, 105.0, MerchantCategory::Electronics),
                tx(3, 95.0, MerchantCategory::Electronics),
                tx(4, 180.0, MerchantCategory::Electronics),
            ],
        };

        apply(&mut transactions);

        let outlier = &transactions.items[3];
        let factor = outlier
            .fraud_factors
            .iter()
            .find_map(|factor| match factor {
                FraudFactor::CategoryPriceDeviation {
                    z_score, weight, ..
                } => Some((*z_score, *weight)),
                _ => None,
            })
            .expect("expected deviation factor");

        assert!(factor.0 >= crate::process::card_statistics::CATEGORY_PRICE_DEVIATION_MIN_Z_SCORE);
        assert!(factor.1 > 0.0);
    }
}
