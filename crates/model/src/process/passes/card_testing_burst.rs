use std::collections::BTreeMap;

use crate::data::channel::Channel;
use crate::data::human_review_status::HumanReviewStatus;
use crate::data::transactions::Transactions;
use crate::process::card_statistics::{
    CARD_TESTING_BURST_MEDIAN_MAX_AMOUNT, CARD_TESTING_BURST_MIN_COUNT, CARD_TESTING_BURST_WINDOW,
    FraudFactor,
};

pub fn apply(transactions: &mut Transactions) {
    let mut by_card: BTreeMap<u64, Vec<usize>> = BTreeMap::new();

    for (idx, tx) in transactions.items.iter().enumerate() {
        by_card.entry(tx.card_id.0).or_default().push(idx);
    }

    for indices in by_card.values_mut() {
        indices.sort_by_key(|idx| transactions.items[*idx].timestamp);

        let online_indices: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|idx| transactions.items[*idx].channel == Channel::Online)
            .collect();

        for start in 0..online_indices.len() {
            let start_idx = online_indices[start];
            let start_ts = transactions.items[start_idx].timestamp;
            let end_ts_limit = start_ts + CARD_TESTING_BURST_WINDOW;

            let mut window_indices: Vec<usize> = Vec::new();
            for idx in online_indices.iter().skip(start).copied() {
                if transactions.items[idx].timestamp >= end_ts_limit {
                    break;
                }
                window_indices.push(idx);
            }

            if window_indices.len() < CARD_TESTING_BURST_MIN_COUNT {
                continue;
            }

            let mut amounts: Vec<f64> = window_indices
                .iter()
                .map(|idx| transactions.items[*idx].amount)
                .collect();
            amounts.sort_by(f64::total_cmp);
            let median_amount = median(&amounts);

            if median_amount >= CARD_TESTING_BURST_MEDIAN_MAX_AMOUNT {
                continue;
            }

            let burst_start =
                transactions.items[*window_indices.first().expect("window has tx")].timestamp;
            let burst_end =
                transactions.items[*window_indices.last().expect("window has tx")].timestamp;
            let max_amount = amounts.into_iter().fold(0.0_f64, f64::max);

            let mut max_gap = chrono::Duration::zero();
            for pair in window_indices.windows(2) {
                let prev = pair[0];
                let next = pair[1];
                let gap = transactions.items[next].timestamp - transactions.items[prev].timestamp;
                if gap > max_gap {
                    max_gap = gap;
                }
            }

            let factor = FraudFactor::CardTestingBurst {
                transaction_count: window_indices.len(),
                burst_start,
                burst_end,
                max_amount,
                max_gap,
            };

            for idx in window_indices {
                transactions.items[idx].human_review_status = HumanReviewStatus::NeedCheck;
                transactions.items[idx].fraud_factors.push(factor.clone());
            }

            // Follow notebook behavior: keep first matching burst window per card.
            break;
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

    fn tx(
        transaction_id: u64,
        card_id: u64,
        timestamp: (i32, u32, u32, u32, u32, u32),
        amount: f64,
        channel: Channel,
    ) -> Transaction {
        let (year, month, day, hour, minute, second) = timestamp;
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("valid date");
        let time = NaiveTime::from_hms_opt(hour, minute, second).expect("valid time");

        Transaction {
            transaction_id: TransactionId(transaction_id),
            timestamp: DateTime::from_naive_utc_and_offset(NaiveDateTime::new(date, time), Utc),
            card_id: CardId(card_id),
            amount,
            merchant_name: "test".to_owned(),
            merchant_category: MerchantCategory::Grocery,
            channel,
            cardholder_country: Country(CountryCode::CA),
            merchant_country: Country(CountryCode::CA),
            device_id: None,
            ip_address: None,
            fraud_factors: Vec::new(),
            human_review_status: HumanReviewStatus::NotNeeded,
        }
    }

    #[test]
    fn marks_burst_transactions_on_same_card() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, 42, (2026, 5, 1, 10, 0, 0), 3.0, Channel::Online),
                tx(2, 42, (2026, 5, 1, 10, 2, 0), 4.0, Channel::Online),
                tx(3, 42, (2026, 5, 1, 10, 4, 30), 5.0, Channel::Online),
                tx(5, 42, (2026, 5, 1, 10, 6, 0), 6.0, Channel::Online),
                tx(6, 42, (2026, 5, 1, 10, 7, 20), 7.0, Channel::Online),
                tx(4, 42, (2026, 5, 1, 11, 30, 0), 40.0, Channel::Online),
            ],
        };

        apply(&mut transactions);

        let burst_hits = transactions
            .items
            .iter()
            .filter(|tx| {
                tx.fraud_factors
                    .iter()
                    .any(|factor| matches!(factor, FraudFactor::CardTestingBurst { .. }))
            })
            .count();

        assert_eq!(burst_hits, 5);
        assert_eq!(
            transactions.items[0].human_review_status,
            HumanReviewStatus::NeedCheck
        );
    }

    #[test]
    fn does_not_mark_when_gap_breaks_run() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, 77, (2026, 5, 1, 10, 0, 0), 3.0, Channel::Online),
                tx(2, 77, (2026, 5, 1, 10, 1, 0), 4.0, Channel::Online),
                tx(3, 77, (2026, 5, 1, 10, 12, 0), 5.0, Channel::Online),
                tx(4, 77, (2026, 5, 1, 10, 13, 0), 5.0, Channel::Online),
                tx(5, 77, (2026, 5, 1, 10, 15, 0), 40.0, Channel::Online),
            ],
        };

        apply(&mut transactions);

        assert!(
            transactions
                .items
                .iter()
                .all(|tx| tx.fraud_factors.is_empty())
        );
    }
}
