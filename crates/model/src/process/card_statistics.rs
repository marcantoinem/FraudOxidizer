use chrono::{DateTime, Duration, Utc};

use crate::data::{
    country::Country, merchant_category::MerchantCategory, transaction::Transaction,
    transactions::Transactions,
};

pub const FRAUD_SCORE_THRESHOLD: f32 = 0.8;
pub const FOREIGN_COUNTRY_WEIGHT: f32 = 0.9;
pub const VACATION_FOREIGN_COUNTRY_WEIGHT: f32 = 0.4;
pub const CARD_TESTING_BURST_WEIGHT: f32 = 0.9;
pub const RISKY_CATEGORY_WEIGHT: f32 = 0.18;
pub const CATEGORY_PRICE_DEVIATION_MIN_Z_SCORE: f64 = 3.0;
pub const CATEGORY_PRICE_DEVIATION_BASE_WEIGHT: f32 = 0.16;
pub const CATEGORY_PRICE_DEVIATION_STEP_WEIGHT: f32 = 0.12;
pub const CATEGORY_PRICE_DEVIATION_MAX_WEIGHT: f32 = 0.75;
pub const HUMAN_REVIEW_SCORE_THRESHOLD_DEFAULT: f32 = 0.55;
pub const CARD_TESTING_BURST_MAX_AMOUNT: f64 = 15.0;
pub const CARD_TESTING_BURST_MIN_COUNT: usize = 3;
pub const CARD_TESTING_BURST_MAX_GAP: Duration = Duration::minutes(10);
pub const VACATION_GAP_THRESHOLD: Duration = Duration::hours(24);
pub const VACATION_SPAN_THRESHOLD: Duration = Duration::hours(24);

#[derive(Debug, Clone, PartialEq)]
pub enum FraudFactor {
    ForeignCountryTrip {
        country: Country,
        primary_country: Country,
        transaction_count: usize,
        trip_start: DateTime<Utc>,
        trip_end: DateTime<Utc>,
        gap_before: Option<Duration>,
        gap_after: Option<Duration>,
        likely_vacation: bool,
    },
    CardTestingBurst {
        transaction_count: usize,
        burst_start: DateTime<Utc>,
        burst_end: DateTime<Utc>,
        max_amount: f64,
        max_gap: Duration,
    },
    InactiveCardTestingBurst {
        transaction_count: usize,
        burst_start: DateTime<Utc>,
        burst_end: DateTime<Utc>,
        max_amount: f64,
        max_gap: Duration,
    },
    RiskyMerchantCategory {
        category: MerchantCategory,
    },
    CategoryPriceDeviation {
        category: MerchantCategory,
        amount: f64,
        average_amount: f64,
        std_deviation: f64,
        z_score: f64,
        weight: f32,
    },
}

impl FraudFactor {
    pub fn weight(&self) -> f32 {
        match self {
            Self::ForeignCountryTrip {
                likely_vacation, ..
            } => {
                if *likely_vacation {
                    VACATION_FOREIGN_COUNTRY_WEIGHT
                } else {
                    FOREIGN_COUNTRY_WEIGHT
                }
            }
            Self::CardTestingBurst { .. } => CARD_TESTING_BURST_WEIGHT,
            Self::InactiveCardTestingBurst { .. } => 0.0,
            Self::RiskyMerchantCategory { .. } => RISKY_CATEGORY_WEIGHT,
            Self::CategoryPriceDeviation { weight, .. } => *weight,
        }
    }

    pub fn reason(&self) -> String {
        match self {
            Self::ForeignCountryTrip {
                country,
                primary_country,
                likely_vacation,
                trip_start,
                trip_end,
                ..
            } => {
                let trip_kind = if *likely_vacation {
                    "likely vacation"
                } else {
                    "short foreign trip"
                };

                format!(
                    "cardholder country {} differs from primary {} ({trip_kind}) between {} and {}",
                    country.0.alpha2(),
                    primary_country.0.alpha2(),
                    trip_start,
                    trip_end,
                )
            }
            Self::CardTestingBurst {
                transaction_count,
                burst_start,
                burst_end,
                max_amount,
                max_gap,
            } => {
                format!(
                    "{transaction_count} rapid small online transactions (<= {:.2}) between {} and {} with max gap {}s",
                    max_amount,
                    burst_start,
                    burst_end,
                    max_gap.num_seconds()
                )
            }
            Self::InactiveCardTestingBurst {
                transaction_count,
                burst_start,
                burst_end,
                max_amount,
                max_gap,
            } => {
                format!(
                    "resolved by human review: {transaction_count} rapid small online transactions (<= {:.2}) between {} and {} with max gap {}s",
                    max_amount,
                    burst_start,
                    burst_end,
                    max_gap.num_seconds()
                )
            }
            Self::RiskyMerchantCategory { category } => {
                format!("merchant category {category:?} is a known higher-risk category")
            }
            Self::CategoryPriceDeviation {
                category,
                amount,
                average_amount,
                std_deviation,
                z_score,
                ..
            } => {
                format!(
                    "amount {:.2} is {:.2} standard deviations above the {:?} average {:.2} (std dev {:.2})",
                    amount, z_score, category, average_amount, std_deviation
                )
            }
        }
    }
}

impl Transaction {
    pub fn fraud_score(&self) -> f32 {
        self.fraud_factors.iter().map(FraudFactor::weight).sum()
    }

    pub fn likely_fraud(&self) -> bool {
        self.fraud_score() >= FRAUD_SCORE_THRESHOLD
    }
}

impl Transactions {
    pub fn card_country_statistics(&mut self) {
        self.apply_fraud_factors();
    }

    pub fn apply_fraud_factors(&mut self) {
        for transaction in &mut self.items {
            transaction.fraud_factors.clear();
        }

        crate::process::passes::foreign_country_trip::apply(self);
        crate::process::passes::card_testing_burst::apply(self);
        crate::process::passes::merchant_category_risk::apply(self);
    }
}

pub fn category_price_deviation_weight(z_score: f64) -> f32 {
    if z_score < CATEGORY_PRICE_DEVIATION_MIN_Z_SCORE {
        return 0.0;
    }

    let weight = CATEGORY_PRICE_DEVIATION_BASE_WEIGHT
        + ((z_score - CATEGORY_PRICE_DEVIATION_MIN_Z_SCORE) as f32)
            * CATEGORY_PRICE_DEVIATION_STEP_WEIGHT;
    weight.min(CATEGORY_PRICE_DEVIATION_MAX_WEIGHT)
}

pub fn is_risky_category(category: MerchantCategory) -> bool {
    matches!(
        category,
        MerchantCategory::GiftCard | MerchantCategory::Electronics
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::data::{
        card_id::CardId, channel::Channel, country::Country,
        human_review_status::HumanReviewStatus, merchant_category::MerchantCategory,
        transaction::Transaction, transaction_id::TransactionId, transactions::Transactions,
    };
    use my_country::Country as CountryCode;

    fn tx(
        card_id: u64,
        timestamp: (i32, u32, u32, u32, u32, u32),
        cardholder_country: CountryCode,
    ) -> Transaction {
        let (year, month, day, hour, minute, second) = timestamp;
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("valid date");
        let time = NaiveTime::from_hms_opt(hour, minute, second).expect("valid time");

        Transaction {
            transaction_id: TransactionId(card_id),
            timestamp: DateTime::from_naive_utc_and_offset(NaiveDateTime::new(date, time), Utc),
            card_id: CardId(card_id),
            amount: 10.0,
            merchant_name: "Test Merchant".to_owned(),
            merchant_category: MerchantCategory::Grocery,
            channel: Channel::InPerson,
            cardholder_country: Country(cardholder_country),
            merchant_country: Country(cardholder_country),
            device_id: None,
            ip_address: None,
            fraud_factors: Vec::new(),
            human_review_status: HumanReviewStatus::NotNeeded,
        }
    }

    fn tx_online(
        card_id: u64,
        timestamp: (i32, u32, u32, u32, u32, u32),
        amount: f64,
    ) -> Transaction {
        let mut tx = tx(card_id, timestamp, CountryCode::CA);
        tx.channel = Channel::Online;
        tx.amount = amount;
        tx
    }

    #[test]
    fn marks_single_foreign_transaction_as_vacation_when_both_gaps_are_large() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, (2026, 5, 1, 8, 0, 0), CountryCode::CA),
                tx(1, (2026, 5, 2, 12, 0, 0), CountryCode::US),
                tx(1, (2026, 5, 3, 20, 0, 0), CountryCode::CA),
            ],
        };

        transactions.apply_fraud_factors();

        let vacation_tx = transactions
            .items
            .iter()
            .find(|tx| tx.cardholder_country == Country(CountryCode::US))
            .expect("expected foreign transaction");

        assert_eq!(vacation_tx.fraud_factors.len(), 1);
        assert!(matches!(
            vacation_tx.fraud_factors[0],
            FraudFactor::ForeignCountryTrip {
                likely_vacation: true,
                ..
            }
        ));
        assert!(!vacation_tx.likely_fraud());
    }

    #[test]
    fn marks_short_foreign_run_as_likely_fraud() {
        let mut transactions = Transactions {
            items: vec![
                tx(1, (2026, 5, 1, 8, 0, 0), CountryCode::CA),
                tx(1, (2026, 5, 1, 9, 0, 0), CountryCode::US),
                tx(1, (2026, 5, 1, 10, 0, 0), CountryCode::US),
                tx(1, (2026, 5, 1, 11, 0, 0), CountryCode::CA),
            ],
        };

        transactions.apply_fraud_factors();

        let fraud_tx = transactions
            .items
            .iter()
            .find(|tx| tx.cardholder_country == Country(CountryCode::US))
            .expect("expected foreign transactions");

        assert!(matches!(
            fraud_tx.fraud_factors[0],
            FraudFactor::ForeignCountryTrip {
                likely_vacation: false,
                ..
            }
        ));
        assert!(fraud_tx.likely_fraud());
        assert!(fraud_tx.fraud_score() >= FRAUD_SCORE_THRESHOLD);
    }

    #[test]
    fn marks_long_multi_transaction_foreign_run_as_vacation() {
        let mut transactions = Transactions {
            items: vec![
                tx(2, (2026, 5, 1, 8, 0, 0), CountryCode::CA),
                tx(2, (2026, 5, 2, 10, 0, 0), CountryCode::CA),
                tx(2, (2026, 5, 6, 8, 44, 56), CountryCode::MX),
                tx(2, (2026, 5, 11, 4, 40, 15), CountryCode::MX),
                tx(2, (2026, 5, 12, 9, 0, 0), CountryCode::CA),
            ],
        };

        transactions.apply_fraud_factors();

        let mx_tx = transactions
            .items
            .iter()
            .find(|tx| tx.cardholder_country == Country(CountryCode::MX))
            .expect("expected MX transaction");

        assert!(matches!(
            mx_tx.fraud_factors[0],
            FraudFactor::ForeignCountryTrip {
                likely_vacation: true,
                ..
            }
        ));
        assert!(!mx_tx.likely_fraud());
    }

    #[test]
    fn marks_rapid_small_online_transactions_as_card_testing_burst() {
        let mut transactions = Transactions {
            items: vec![
                tx_online(10, (2026, 5, 1, 10, 0, 0), 2.15),
                tx_online(10, (2026, 5, 1, 10, 1, 20), 4.00),
                tx_online(10, (2026, 5, 1, 10, 3, 10), 3.25),
                tx_online(10, (2026, 5, 1, 11, 30, 0), 45.0),
            ],
        };

        transactions.apply_fraud_factors();

        let burst_count = transactions
            .items
            .iter()
            .filter(|tx| {
                tx.fraud_factors.iter().any(|factor| {
                    matches!(
                        factor,
                        FraudFactor::CardTestingBurst {
                            transaction_count: 3,
                            ..
                        }
                    )
                })
            })
            .count();

        assert_eq!(burst_count, 3);
        assert!(transactions.items[0].likely_fraud());
        assert_eq!(
            transactions.items[0].human_review_status,
            HumanReviewStatus::NeedCheck
        );
    }

    #[test]
    fn does_not_mark_non_online_or_large_transactions_as_card_testing_burst() {
        let mut transactions = Transactions {
            items: vec![
                tx_online(11, (2026, 5, 1, 10, 0, 0), 2.15),
                tx_online(11, (2026, 5, 1, 10, 1, 20), 20.00),
                tx(11, (2026, 5, 1, 10, 2, 10), CountryCode::CA),
                tx_online(11, (2026, 5, 1, 10, 3, 10), 3.25),
            ],
        };

        transactions.apply_fraud_factors();

        assert!(transactions.items.iter().all(|tx| {
            tx.fraud_factors
                .iter()
                .all(|factor| !matches!(factor, FraudFactor::CardTestingBurst { .. }))
        }));
    }

    #[test]
    fn inactive_burst_has_zero_weight() {
        let mut transaction = tx_online(12, (2026, 5, 1, 10, 0, 0), 3.0);
        transaction
            .fraud_factors
            .push(FraudFactor::InactiveCardTestingBurst {
                transaction_count: 3,
                burst_start: transaction.timestamp,
                burst_end: transaction.timestamp + Duration::minutes(2),
                max_amount: 4.5,
                max_gap: Duration::minutes(1),
            });

        assert_eq!(transaction.fraud_factors[0].weight(), 0.0);
        assert!(!transaction.likely_fraud());
    }

    #[test]
    fn risky_category_weight_stays_small() {
        let factor = FraudFactor::RiskyMerchantCategory {
            category: MerchantCategory::GiftCard,
        };

        assert_eq!(factor.weight(), RISKY_CATEGORY_WEIGHT);
        assert!(factor.weight() < FRAUD_SCORE_THRESHOLD);
    }

    #[test]
    fn category_deviation_weight_increases_with_distance() {
        let below_cutoff = category_price_deviation_weight(2.5);
        let low = category_price_deviation_weight(CATEGORY_PRICE_DEVIATION_MIN_Z_SCORE);
        let higher = category_price_deviation_weight(4.5);
        let capped = category_price_deviation_weight(20.0);

        assert_eq!(below_cutoff, 0.0);
        assert!(low > 0.0);
        assert!(higher > low);
        assert_eq!(capped, CATEGORY_PRICE_DEVIATION_MAX_WEIGHT);
    }
}
