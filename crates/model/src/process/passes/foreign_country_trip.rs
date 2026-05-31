use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::data::channel::Channel;
use crate::data::human_review_status::HumanReviewStatus;
use crate::data::{country::Country, transactions::Transactions};
use crate::process::card_statistics::FraudFactor;

pub fn apply(transactions: &mut Transactions) {
    let mut by_card: BTreeMap<u64, Vec<usize>> = BTreeMap::new();

    for (idx, tx) in transactions.items.iter().enumerate() {
        by_card.entry(tx.card_id.0).or_default().push(idx);
    }

    for indices in by_card.values_mut() {
        indices.sort_by_key(|idx| transactions.items[*idx].timestamp);

        let physical_indices: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|idx| {
                matches!(
                    transactions.items[*idx].channel,
                    Channel::InPerson | Channel::Atm
                )
            })
            .collect();
        if physical_indices.is_empty() {
            continue;
        }

        let primary_country = primary_country_for_card(transactions, &physical_indices)
            .expect("each card has at least one transaction");

        for tx_idx in physical_indices {
            let transaction = &transactions.items[tx_idx];
            if transaction.merchant_country == primary_country {
                continue;
            }

            let factor = FraudFactor::ForeignCountryTrip {
                country: transaction.merchant_country,
                primary_country,
                transaction_count: 1,
                trip_start: transaction.timestamp,
                trip_end: transaction.timestamp,
                gap_before: None,
                gap_after: None,
                likely_vacation: false,
            };

            transactions.items[tx_idx].human_review_status = HumanReviewStatus::NeedCheck;
            transactions.items[tx_idx].fraud_factors.push(factor);
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
