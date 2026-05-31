use std::collections::BTreeMap;

use crate::data::transactions::Transactions;
use crate::process::card_statistics::{
    CASHOUT_MIN_AMOUNT, FraudFactor, category_price_deviation_weight, is_risky_category,
};

pub fn apply(transactions: &mut Transactions) {
    let mut category_stats: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();

    for transaction in &transactions.items {
        category_stats
            .entry(format!("{:?}", transaction.merchant_category))
            .and_modify(|(count, sum, sum_sq)| {
                *count += 1;
                *sum += transaction.amount;
                *sum_sq += transaction.amount * transaction.amount;
            })
            .or_insert((
                1,
                transaction.amount,
                transaction.amount * transaction.amount,
            ));
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

        let category_key = format!("{:?}", transaction.merchant_category);
        let Some((count, sum, sum_sq)) = category_stats.get(&category_key).copied() else {
            continue;
        };
        if count < 2 {
            continue;
        }

        let average_amount = sum / count as f64;
        if transaction.amount <= average_amount {
            continue;
        }

        let variance = (sum_sq / count as f64) - (average_amount * average_amount);
        let std_deviation = variance.max(0.0).sqrt();
        if std_deviation <= f64::EPSILON {
            continue;
        }

        let z_score = (transaction.amount - average_amount) / std_deviation;
        let weight = category_price_deviation_weight(z_score);
        if weight <= 0.0 {
            continue;
        }

        transaction
            .fraud_factors
            .push(FraudFactor::CategoryPriceDeviation {
                category: transaction.merchant_category,
                amount: transaction.amount,
                average_amount,
                std_deviation,
                z_score,
                weight,
            });
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
