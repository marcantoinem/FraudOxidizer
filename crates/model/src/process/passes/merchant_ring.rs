use std::collections::{BTreeMap, BTreeSet};

use crate::data::transactions::Transactions;
use crate::process::card_statistics::{
    FraudFactor, MERCHANT_RING_MIN_AMOUNT, MERCHANT_RING_MIN_DISTINCT_CARDS,
    MERCHANT_RING_MIN_OUTLIERS, MERCHANT_RING_MULTIPLIER,
};

pub fn apply(transactions: &mut Transactions) {
    let mut by_merchant: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (idx, transaction) in transactions.items.iter().enumerate() {
        by_merchant
            .entry(transaction.merchant_name.clone())
            .or_default()
            .push(idx);
    }

    for (merchant_name, indices) in by_merchant {
        if indices.len() < MERCHANT_RING_MIN_OUTLIERS {
            continue;
        }

        let mut amounts: Vec<f64> = indices
            .iter()
            .map(|idx| transactions.items[*idx].amount)
            .collect();
        amounts.sort_by(f64::total_cmp);

        let merchant_median = median(&amounts);
        if merchant_median <= f64::EPSILON {
            continue;
        }

        let outlier_indices: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|idx| {
                let amount = transactions.items[*idx].amount;
                amount > merchant_median * MERCHANT_RING_MULTIPLIER && amount > MERCHANT_RING_MIN_AMOUNT
            })
            .collect();

        if outlier_indices.len() < MERCHANT_RING_MIN_OUTLIERS {
            continue;
        }

        let distinct_card_count = outlier_indices
            .iter()
            .map(|idx| transactions.items[*idx].card_id.0)
            .collect::<BTreeSet<_>>()
            .len();

        if distinct_card_count < MERCHANT_RING_MIN_DISTINCT_CARDS {
            continue;
        }

        let outlier_count = outlier_indices.len();

        for idx in outlier_indices {
            let amount = transactions.items[idx].amount;
            let ratio = amount / merchant_median;
            transactions.items[idx]
                .fraud_factors
                .push(FraudFactor::MerchantRing {
                    merchant_name: merchant_name.clone(),
                    amount,
                    merchant_median,
                    ratio,
                    outlier_count,
                    distinct_card_count,
                });
        }
    }
}

fn median(sorted_values: &[f64]) -> f64 {
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

    fn tx(transaction_id: u64, card_id: u64, merchant_name: &str, amount: f64) -> Transaction {
        Transaction {
            transaction_id: TransactionId(transaction_id),
            timestamp: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date"),
                    NaiveTime::from_hms_opt(10, 0, 0).expect("valid time"),
                ),
                Utc,
            ),
            card_id: CardId(card_id),
            amount,
            merchant_name: merchant_name.to_owned(),
            merchant_category: MerchantCategory::OnlineRetail,
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
    fn marks_cross_card_merchant_ring_outliers() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, 1, "ring", 20.0),
                tx(2, 2, "ring", 25.0),
                tx(3, 3, "ring", 22.0),
                tx(4, 4, "ring", 300.0),
                tx(5, 5, "ring", 320.0),
                tx(6, 6, "ring", 350.0),
            ],
        };

        apply(&mut transactions);

        let outliers = transactions
            .items
            .iter()
            .filter(|tx| {
                tx.fraud_factors
                    .iter()
                    .any(|factor| matches!(factor, FraudFactor::MerchantRing { .. }))
            })
            .count();

        assert_eq!(outliers, 3);
    }
}
