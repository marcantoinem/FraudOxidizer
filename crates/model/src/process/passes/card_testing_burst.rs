use std::collections::BTreeMap;

use crate::data::channel::Channel;
use crate::data::human_review_status::HumanReviewStatus;
use crate::data::transactions::Transactions;
use crate::process::card_statistics::{
    CARD_TESTING_BURST_MAX_AMOUNT, CARD_TESTING_BURST_MAX_GAP, CARD_TESTING_BURST_MIN_COUNT,
    FraudFactor,
};

pub fn apply(transactions: &mut Transactions) {
    let mut by_card: BTreeMap<u64, Vec<usize>> = BTreeMap::new();

    for (idx, tx) in transactions.items.iter().enumerate() {
        by_card.entry(tx.card_id.0).or_default().push(idx);
    }

    for indices in by_card.values_mut() {
        indices.sort_by_key(|idx| transactions.items[*idx].timestamp);

        let mut run_start: Option<usize> = None;

        for position in 0..indices.len() {
            let tx_idx = indices[position];
            let tx = &transactions.items[tx_idx];
            let qualifies =
                tx.channel == Channel::Online && tx.amount <= CARD_TESTING_BURST_MAX_AMOUNT;

            if !qualifies {
                flush_run(transactions, indices, run_start, position.saturating_sub(1));
                run_start = None;
                continue;
            }

            if let Some(start) = run_start {
                let prev_idx = indices[position - 1];
                let gap = tx.timestamp - transactions.items[prev_idx].timestamp;
                if gap > CARD_TESTING_BURST_MAX_GAP {
                    flush_run(transactions, indices, Some(start), position - 1);
                    run_start = Some(position);
                }
            } else {
                run_start = Some(position);
            }
        }

        flush_run(
            transactions,
            indices,
            run_start,
            indices.len().saturating_sub(1),
        );
    }
}

fn flush_run(
    transactions: &mut Transactions,
    indices: &[usize],
    run_start: Option<usize>,
    run_end: usize,
) {
    let Some(start) = run_start else {
        return;
    };

    if run_end < start {
        return;
    }

    let count = run_end - start + 1;
    if count < CARD_TESTING_BURST_MIN_COUNT {
        return;
    }

    let first_idx = indices[start];
    let last_idx = indices[run_end];
    let burst_start = transactions.items[first_idx].timestamp;
    let burst_end = transactions.items[last_idx].timestamp;

    let mut max_amount: f64 = 0.0;
    let mut max_gap = chrono::Duration::zero();

    for pos in start..=run_end {
        let idx = indices[pos];
        max_amount = max_amount.max(transactions.items[idx].amount);
        if pos > start {
            let prev = indices[pos - 1];
            let gap = transactions.items[idx].timestamp - transactions.items[prev].timestamp;
            if gap > max_gap {
                max_gap = gap;
            }
        }
    }

    let factor = FraudFactor::CardTestingBurst {
        transaction_count: count,
        burst_start,
        burst_end,
        max_amount,
        max_gap,
    };

    for pos in start..=run_end {
        let idx = indices[pos];
        transactions.items[idx].human_review_status = HumanReviewStatus::NeedCheck;
        transactions.items[idx].fraud_factors.push(factor.clone());
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

        assert_eq!(burst_hits, 3);
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
