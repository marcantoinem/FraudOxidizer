use std::collections::BTreeMap;

use crate::data::transactions::Transactions;
use crate::process::card_statistics::{
    CARD_AMOUNT_DEVIATION_MIN_MEAN_MULTIPLIER, FraudFactor, card_amount_deviation_weight,
};

pub fn apply(transactions: &mut Transactions) {
    let mut by_card: BTreeMap<u64, (usize, f64, f64)> = BTreeMap::new();

    for transaction in &transactions.items {
        by_card
            .entry(transaction.card_id.0)
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
        let Some((count, sum, sum_sq)) = by_card.get(&transaction.card_id.0).copied() else {
            continue;
        };

        let peer_count = count.saturating_sub(1);
        if peer_count < 3 {
            continue;
        }

        let peer_sum = sum - transaction.amount;
        let average_amount = peer_sum / peer_count as f64;
        let peer_sum_sq = sum_sq - transaction.amount * transaction.amount;
        let peer_variance = (peer_sum_sq / peer_count as f64) - (average_amount * average_amount);
        let std_deviation = peer_variance.max(0.0).sqrt();

        if std_deviation <= f64::EPSILON || transaction.amount <= average_amount {
            continue;
        }

        if transaction.amount < average_amount * CARD_AMOUNT_DEVIATION_MIN_MEAN_MULTIPLIER {
            continue;
        }

        let z_score = (transaction.amount - average_amount) / std_deviation;
        let weight = card_amount_deviation_weight(z_score);
        if weight <= 0.0 {
            continue;
        }

        transaction
            .fraud_factors
            .push(FraudFactor::CardAmountDeviation {
                card_id: transaction.card_id.0,
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

    fn tx(transaction_id: u64, card_id: u64, amount: f64) -> Transaction {
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
            merchant_name: "merchant".to_owned(),
            merchant_category: MerchantCategory::Grocery,
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
    fn adds_card_amount_deviation_factor_for_large_outlier() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, 55, 20.0),
                tx(2, 55, 22.0),
                tx(3, 55, 18.0),
                tx(4, 55, 21.0),
                tx(5, 55, 220.0),
            ],
        };

        apply(&mut transactions);

        let outlier = &transactions.items[4];
        let factor = outlier
            .fraud_factors
            .iter()
            .find_map(|factor| match factor {
                FraudFactor::CardAmountDeviation {
                    card_id,
                    z_score,
                    weight,
                    ..
                } => Some((*card_id, *z_score, *weight)),
                _ => None,
            })
            .expect("expected card amount deviation factor");

        assert_eq!(factor.0, 55);
        assert!(factor.1 >= crate::process::card_statistics::CARD_AMOUNT_DEVIATION_MIN_Z_SCORE);
        assert!(factor.2 > 0.0);
    }

    #[test]
    fn does_not_add_factor_when_amount_is_not_ten_times_mean() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, 77, 1.0),
                tx(2, 77, 1.1),
                tx(3, 77, 0.9),
                tx(4, 77, 1.2),
                tx(5, 77, 5.0),
            ],
        };

        apply(&mut transactions);

        let outlier = &transactions.items[4];
        assert!(
            outlier
                .fraud_factors
                .iter()
                .all(|factor| !matches!(factor, FraudFactor::CardAmountDeviation { .. }))
        );
    }
}
