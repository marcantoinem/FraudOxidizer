use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::data::human_review_status::HumanReviewStatus;
use crate::data::{country::Country, transactions::Transactions};
use crate::process::card_statistics::{
    FraudFactor, VACATION_GAP_THRESHOLD, VACATION_SPAN_THRESHOLD,
};

pub fn apply(transactions: &mut Transactions) {
    let mut by_card: BTreeMap<u64, Vec<usize>> = BTreeMap::new();

    for (idx, tx) in transactions.items.iter().enumerate() {
        by_card.entry(tx.card_id.0).or_default().push(idx);
    }

    for indices in by_card.values_mut() {
        indices.sort_by_key(|idx| transactions.items[*idx].timestamp);

        let primary_country = primary_country_for_card(transactions, indices)
            .expect("each card has at least one transaction");

        let mut cursor = 0;
        while cursor < indices.len() {
            let tx_idx = indices[cursor];
            let country = transactions.items[tx_idx].cardholder_country;
            if country == primary_country {
                cursor += 1;
                continue;
            }

            let start = cursor;
            let mut end = cursor;
            while end + 1 < indices.len() {
                let next_country = transactions.items[indices[end + 1]].cardholder_country;
                if next_country != country {
                    break;
                }
                end += 1;
            }

            let start_idx = indices[start];
            let end_idx = indices[end];
            let start_ts = transactions.items[start_idx].timestamp;
            let end_ts = transactions.items[end_idx].timestamp;
            let gap_before = if start > 0 {
                Some(start_ts - transactions.items[indices[start - 1]].timestamp)
            } else {
                None
            };
            let gap_after = if end + 1 < indices.len() {
                Some(transactions.items[indices[end + 1]].timestamp - end_ts)
            } else {
                None
            };
            let transaction_count = end - start + 1;
            let span = end_ts - start_ts;
            let likely_vacation = if transaction_count == 1 {
                gap_before.is_some_and(|gap| gap >= VACATION_GAP_THRESHOLD)
                    && gap_after.is_some_and(|gap| gap >= VACATION_GAP_THRESHOLD)
            } else {
                span >= VACATION_SPAN_THRESHOLD
            };

            let factor = FraudFactor::ForeignCountryTrip {
                country,
                primary_country,
                transaction_count,
                trip_start: start_ts,
                trip_end: end_ts,
                gap_before,
                gap_after,
                likely_vacation,
            };

            for idx in &indices[start..=end] {
                transactions.items[*idx].human_review_status = HumanReviewStatus::NeedCheck;
                transactions.items[*idx].fraud_factors.push(factor.clone());
            }

            cursor = end + 1;
        }
    }
}

fn primary_country_for_card(transactions: &Transactions, indices: &[usize]) -> Option<Country> {
    let mut counts: BTreeMap<String, (Country, usize, DateTime<Utc>)> = BTreeMap::new();

    for idx in indices {
        let tx = &transactions.items[*idx];
        let country = tx.cardholder_country;
        let key = country.0.alpha2().to_owned();
        counts
            .entry(key)
            .and_modify(|(_, count, _)| *count += 1)
            .or_insert((country, 1, tx.timestamp));
    }

    let mut best: Option<(Country, usize, DateTime<Utc>)> = None;
    for (country, count, first_seen) in counts.into_values() {
        best = match best {
            None => Some((country, count, first_seen)),
            Some((best_country, best_count, best_first_seen)) => {
                if count > best_count || (count == best_count && first_seen < best_first_seen) {
                    Some((country, count, first_seen))
                } else {
                    Some((best_country, best_count, best_first_seen))
                }
            }
        };
    }

    best.map(|(country, _, _)| country)
}
