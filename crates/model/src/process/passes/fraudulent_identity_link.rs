use crate::data::human_review_status::HumanReviewStatus;
use crate::data::transaction::Transaction;
use crate::data::transactions::Transactions;
use crate::process::card_statistics::{
    FRAUD_SCORE_THRESHOLD, FRAUDULENT_IDENTITY_LINK_WEIGHT_CONFIRMED,
    FRAUDULENT_IDENTITY_LINK_WEIGHT_LIKELY, FraudFactor,
};

type IdentitySignal = (u64, Option<String>, Option<std::net::IpAddr>, bool, bool);

pub fn apply(transactions: &mut Transactions) {
    apply_to_items(&mut transactions.items);
}

pub fn apply_to_items(items: &mut [Transaction]) {
    for transaction in items.iter_mut() {
        transaction
            .fraud_factors
            .retain(|factor| !matches!(factor, FraudFactor::FraudulentIdentityLink { .. }));
    }

    let signals: Vec<IdentitySignal> = items
        .iter()
        .map(|transaction| {
            (
                transaction.transaction_id.0,
                transaction.device_id.clone(),
                transaction.ip_address,
                matches!(
                    transaction.human_review_status,
                    HumanReviewStatus::TruePositive
                ),
                !matches!(
                    transaction.human_review_status,
                    HumanReviewStatus::FalsePositive
                ) && transaction.fraud_score() >= FRAUD_SCORE_THRESHOLD,
            )
        })
        .collect();

    for transaction in items.iter_mut() {
        let mut matched_confirmed_count = 0usize;
        let mut matched_likely_count = 0usize;

        for (source_tx_id, source_device, source_ip, source_confirmed, source_likely) in &signals {
            if *source_tx_id == transaction.transaction_id.0 {
                continue;
            }

            let same_device = match (&transaction.device_id, source_device) {
                (Some(device), Some(source)) => device == source,
                _ => false,
            };
            let same_ip = match (transaction.ip_address, *source_ip) {
                (Some(ip), Some(source)) => ip == source,
                _ => false,
            };

            if !same_device && !same_ip {
                continue;
            }

            if *source_confirmed {
                matched_confirmed_count += usize::from(same_device) + usize::from(same_ip);
            } else if *source_likely {
                matched_likely_count += usize::from(same_device) + usize::from(same_ip);
            }
        }

        if matched_confirmed_count == 0 && matched_likely_count == 0 {
            continue;
        }

        let weight = if matched_confirmed_count > 0 {
            FRAUDULENT_IDENTITY_LINK_WEIGHT_CONFIRMED
        } else {
            FRAUDULENT_IDENTITY_LINK_WEIGHT_LIKELY
        };

        transaction
            .fraud_factors
            .push(FraudFactor::FraudulentIdentityLink {
                device_id: transaction.device_id.clone(),
                ip_address: transaction.ip_address,
                matched_confirmed_count,
                matched_likely_count,
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
    use std::net::IpAddr;

    fn tx(
        transaction_id: u64,
        amount: f64,
        device_id: Option<&str>,
        ip_address: Option<&str>,
        review: HumanReviewStatus,
    ) -> Transaction {
        Transaction {
            transaction_id: TransactionId(transaction_id),
            timestamp: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date"),
                    NaiveTime::from_hms_opt(10, 0, 0).expect("valid time"),
                ),
                Utc,
            ),
            card_id: CardId(transaction_id),
            amount,
            merchant_name: "merchant".to_owned(),
            merchant_category: MerchantCategory::Grocery,
            channel: Channel::Online,
            cardholder_country: Country(CountryCode::CA),
            merchant_country: Country(CountryCode::CA),
            device_id: device_id.map(str::to_owned),
            ip_address: ip_address.map(|ip| ip.parse::<IpAddr>().expect("valid ip")),
            fraud_factors: Vec::new(),
            human_review_status: review,
        }
    }

    #[test]
    fn promotes_weight_when_identity_is_human_confirmed() {
        let mut items = vec![
            tx(
                1,
                10.0,
                Some("dev-1"),
                Some("10.0.0.1"),
                HumanReviewStatus::TruePositive,
            ),
            tx(
                2,
                12.0,
                Some("dev-1"),
                Some("10.0.0.2"),
                HumanReviewStatus::NotNeeded,
            ),
        ];

        apply_to_items(&mut items);

        let factor = items[1]
            .fraud_factors
            .iter()
            .find_map(|factor| match factor {
                FraudFactor::FraudulentIdentityLink { weight, .. } => Some(*weight),
                _ => None,
            })
            .expect("expected identity link factor");

        assert_eq!(factor, FRAUDULENT_IDENTITY_LINK_WEIGHT_CONFIRMED);
    }

    #[test]
    fn keeps_lower_weight_for_unconfirmed_likely_identity() {
        let mut items = vec![
            tx(1, 5000.0, Some("dev-1"), None, HumanReviewStatus::NeedCheck),
            tx(2, 10.0, Some("dev-1"), None, HumanReviewStatus::NotNeeded),
        ];

        items[0].fraud_factors.push(FraudFactor::MerchantRing {
            merchant_name: "m".to_owned(),
            amount: 5000.0,
            merchant_median: 100.0,
            ratio: 50.0,
            outlier_count: 3,
            distinct_card_count: 3,
        });

        apply_to_items(&mut items);

        let factor = items[1]
            .fraud_factors
            .iter()
            .find_map(|factor| match factor {
                FraudFactor::FraudulentIdentityLink { weight, .. } => Some(*weight),
                _ => None,
            })
            .expect("expected identity link factor");

        assert_eq!(factor, FRAUDULENT_IDENTITY_LINK_WEIGHT_LIKELY);
    }

    #[test]
    fn does_not_self_match_single_transaction() {
        let mut items = vec![tx(
            1,
            1000.0,
            Some("dev-1"),
            Some("10.0.0.1"),
            HumanReviewStatus::TruePositive,
        )];

        apply_to_items(&mut items);

        assert!(
            items[0]
                .fraud_factors
                .iter()
                .all(|factor| !matches!(factor, FraudFactor::FraudulentIdentityLink { .. }))
        );
    }
}
