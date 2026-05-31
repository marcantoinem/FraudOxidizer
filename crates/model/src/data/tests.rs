use std::{
    fs,
    path::PathBuf,
    process,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    card_id::CardId, channel::Channel, country::Country, human_review_status::HumanReviewStatus,
    merchant_category::MerchantCategory, optional_ip_addr, optional_string, parse_amount,
    parse_prefixed_number, parse_timestamp, transaction::Transaction,
    transaction_id::TransactionId, transactions::Transactions,
};
use crate::ParseCsvError;
use crate::process::card_statistics::FraudFactor;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use my_country::Country as CountryCode;

fn write_csv(contents: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_nanos();

    let path = std::env::temp_dir().join(format!(
        "valsoft-fraud-detector-{process_id}-{unique_suffix}.csv",
        process_id = process::id()
    ));

    fs::write(&path, contents).expect("failed to write test csv");
    path
}

fn valid_csv_row(overrides: &[(usize, &str)]) -> String {
    let mut columns = vec![
        "tx_000001",
        "2026-05-05T18:39:29",
        "card_000",
        "115.44",
        "Bell Canada",
        "utilities",
        "online",
        "CA",
        "CA",
        "dev_bc0178b6",
        "24.114.150.203",
    ];

    for (index, value) in overrides {
        columns[*index] = value;
    }

    columns.join(",")
}

#[test]
fn parse_prefixed_number_rejects_bad_prefix() {
    let error = parse_prefixed_number("order_42", "tx_").expect_err("expected invalid prefix");

    assert!(matches!(
        error,
        ParseCsvError::InvalidPrefix {
            field: "transaction_id",
            expected_prefix: "tx_",
            value,
        } if value == "order_42"
    ));
}

#[test]
fn parse_prefixed_number_rejects_bad_number() {
    let error =
        parse_prefixed_number("tx_not-a-number", "tx_").expect_err("expected invalid number");

    assert!(matches!(
        error,
        ParseCsvError::InvalidNumber {
            field: "transaction_id",
            value,
        } if value == "tx_not-a-number"
    ));
}

#[test]
fn card_id_rejects_invalid_prefix_and_number() {
    let prefix_error = CardId::from_str("transaction_1").expect_err("expected invalid card prefix");
    let number_error =
        CardId::from_str("card_not-a-number").expect_err("expected invalid card number");

    assert!(matches!(
        prefix_error,
        ParseCsvError::InvalidPrefix {
            field: "card_id",
            expected_prefix: "card_",
            value,
        } if value == "transaction_1"
    ));
    assert!(matches!(
        number_error,
        ParseCsvError::InvalidNumber {
            field: "card_id",
            value,
        } if value == "card_not-a-number"
    ));
}

#[test]
fn transaction_id_rejects_invalid_prefix_and_number() {
    let prefix_error =
        TransactionId::from_str("order_1").expect_err("expected invalid transaction prefix");
    let number_error = TransactionId::from_str("tx_not-a-number")
        .expect_err("expected invalid transaction number");

    assert!(matches!(
        prefix_error,
        ParseCsvError::InvalidPrefix {
            field: "transaction_id",
            expected_prefix: "tx_",
            value,
        } if value == "order_1"
    ));
    assert!(matches!(
        number_error,
        ParseCsvError::InvalidNumber {
            field: "transaction_id",
            value,
        } if value == "tx_not-a-number"
    ));
}

#[test]
fn enum_parsers_reject_invalid_values() {
    let channel_error = Channel::from_str("telepathy").expect_err("expected invalid channel");
    let country_error = Country::from_str("ZZ").expect_err("expected invalid country");
    let merchant_category_error =
        MerchantCategory::from_str("books").expect_err("expected invalid merchant category");

    assert!(matches!(
        channel_error,
        ParseCsvError::InvalidEnumValue {
            field: "channel",
            value,
        } if value == "telepathy"
    ));
    assert!(matches!(
        country_error,
        ParseCsvError::InvalidEnumValue {
            field: "country",
            value,
        } if value == "ZZ"
    ));
    assert!(matches!(
        merchant_category_error,
        ParseCsvError::InvalidEnumValue {
            field: "merchant_category",
            value,
        } if value == "books"
    ));
}

#[test]
fn country_parser_accepts_codes_beyond_the_old_hand_written_list() {
    let country = Country::from_str("PT").expect("expected supported country code");

    assert_eq!(country, Country(CountryCode::PT));
}

#[test]
fn helper_parsers_reject_invalid_values() {
    let amount_error = parse_amount("not-a-float").expect_err("expected invalid float");
    let timestamp_error =
        parse_timestamp("2026-99-05T18:39:29").expect_err("expected invalid datetime");
    let ip_error = optional_ip_addr("not-an-ip").expect_err("expected invalid ip address");

    assert!(matches!(
        amount_error,
        ParseCsvError::InvalidFloat {
            field: "amount",
            value,
        } if value == "not-a-float"
    ));
    assert!(matches!(
        timestamp_error,
        ParseCsvError::InvalidDateTime {
            field: "timestamp",
            value,
        } if value == "2026-99-05T18:39:29"
    ));
    assert!(matches!(
        ip_error,
        ParseCsvError::InvalidIpAddress {
            field: "ip_address",
            value,
        } if value == "not-an-ip"
    ));
}

#[test]
fn helper_parsers_accept_empty_optional_values() {
    assert_eq!(optional_string(""), None);
    assert_eq!(
        optional_ip_addr("").expect("empty ip should be allowed"),
        None
    );
}

#[test]
fn transactions_parse_csv_propagates_field_errors() {
    let csv = format!("header\n{}", valid_csv_row(&[(0, "order_42")]));
    let path = write_csv(&csv);

    let error = Transactions::parse_csv(&path).expect_err("expected invalid prefix");

    let _ = fs::remove_file(path);

    assert!(matches!(
        error,
        ParseCsvError::InvalidPrefix {
            field: "transaction_id",
            expected_prefix: "tx_",
            value,
        } if value == "order_42"
    ));
}

#[test]
fn transactions_parse_csv_propagates_column_count_error() {
    let csv = format!(
        "header\n{}",
        valid_csv_row(&[]).trim_end_matches(",dev_bc0178b6,24.114.150.203")
    );
    let path = write_csv(&csv);

    let error = Transactions::parse_csv(&path).expect_err("expected invalid column count");

    let _ = fs::remove_file(path);

    assert!(matches!(
        error,
        ParseCsvError::InvalidColumnCount {
            expected: 11,
            found: 9,
            line: 2,
        }
    ));
}

#[test]
fn transactions_parse_csv_accepts_valid_row() {
    let csv = format!("header\n{}", valid_csv_row(&[]));
    let path = write_csv(&csv);

    let transactions = Transactions::parse_csv(&path).expect("expected valid csv");

    let _ = fs::remove_file(path);

    assert_eq!(transactions.items.len(), 1);
}

#[test]
fn transactions_parse_csv_accepts_new_country_codes() {
    let csv = format!("header\n{}", valid_csv_row(&[(7, "PT"), (8, "PT")]));
    let path = write_csv(&csv);

    let transactions =
        Transactions::parse_csv(&path).expect("expected valid csv with PT country codes");

    let _ = fs::remove_file(path);

    assert_eq!(transactions.items.len(), 1);
    assert_eq!(
        transactions.items[0].cardholder_country,
        Country(CountryCode::PT)
    );
    assert_eq!(
        transactions.items[0].merchant_country,
        Country(CountryCode::PT)
    );
}

#[test]
fn transactions_parse_csv_accepts_project_csv_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../transactions.csv");

    let transactions =
        Transactions::parse_csv(&path).expect("expected repository transactions.csv to be valid");

    assert!(!transactions.items.is_empty());
}

#[test]
fn transactions_export_csv_appends_likely_fraud_column() {
    let timestamp = DateTime::from_naive_utc_and_offset(
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 5, 5).expect("valid date"),
            NaiveTime::from_hms_opt(18, 39, 29).expect("valid time"),
        ),
        Utc,
    );

    let transactions = Transactions {
        items: vec![
            Transaction {
                transaction_id: TransactionId(1),
                timestamp,
                card_id: CardId(0),
                amount: 115.44,
                merchant_name: "Bell Canada".to_owned(),
                merchant_category: MerchantCategory::Utilities,
                channel: Channel::Online,
                cardholder_country: Country(CountryCode::CA),
                merchant_country: Country(CountryCode::CA),
                device_id: Some("dev_bc0178b6".to_owned()),
                ip_address: Some("24.114.150.203".parse().expect("valid ip")),
                fraud_factors: vec![FraudFactor::ForeignCountryTrip {
                    country: Country(CountryCode::US),
                    primary_country: Country(CountryCode::CA),
                    transaction_count: 1,
                    trip_start: timestamp,
                    trip_end: timestamp,
                    gap_before: None,
                    gap_after: None,
                    likely_vacation: false,
                }],
                human_review_status: HumanReviewStatus::NotNeeded,
            },
            Transaction {
                transaction_id: TransactionId(2),
                timestamp: DateTime::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2026, 5, 5).expect("valid date"),
                        NaiveTime::from_hms_opt(19, 0, 0).expect("valid time"),
                    ),
                    Utc,
                ),
                card_id: CardId(1),
                amount: 10.0,
                merchant_name: "Local Shop".to_owned(),
                merchant_category: MerchantCategory::Grocery,
                channel: Channel::InPerson,
                cardholder_country: Country(CountryCode::CA),
                merchant_country: Country(CountryCode::CA),
                device_id: None,
                ip_address: None,
                fraud_factors: Vec::new(),
                human_review_status: HumanReviewStatus::TruePositive,
            },
        ],
    };

    let exported = transactions.export_csv_content();

    assert!(exported.starts_with("transaction_id,timestamp,card_id,amount,merchant_name,merchant_category,channel,cardholder_country,merchant_country,device_id,ip_address,likely_fraud\n"));
    assert!(exported.contains("tx_000001,2026-05-05T18:39:29,card_000,115.44,Bell Canada,utilities,online,CA,CA,dev_bc0178b6,24.114.150.203,true"));
    assert!(exported.contains(
        "tx_000002,2026-05-05T19:00:00,card_001,10,Local Shop,grocery,in_person,CA,CA,,,true"
    ));
}

#[test]
fn transactions_export_csv_respects_false_positive_review() {
    let timestamp = DateTime::from_naive_utc_and_offset(
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 5, 5).expect("valid date"),
            NaiveTime::from_hms_opt(20, 0, 0).expect("valid time"),
        ),
        Utc,
    );

    let transactions = Transactions {
        items: vec![Transaction {
            transaction_id: TransactionId(3),
            timestamp,
            card_id: CardId(2),
            amount: 250.0,
            merchant_name: "Suspicious Merchant".to_owned(),
            merchant_category: MerchantCategory::Electronics,
            channel: Channel::Online,
            cardholder_country: Country(CountryCode::CA),
            merchant_country: Country(CountryCode::US),
            device_id: Some("dev_x".to_owned()),
            ip_address: Some("203.0.113.10".parse().expect("valid ip")),
            fraud_factors: vec![FraudFactor::ForeignCountryTrip {
                country: Country(CountryCode::US),
                primary_country: Country(CountryCode::CA),
                transaction_count: 1,
                trip_start: timestamp,
                trip_end: timestamp,
                gap_before: None,
                gap_after: None,
                likely_vacation: false,
            }],
            human_review_status: HumanReviewStatus::FalsePositive,
        }],
    };

    let exported = transactions.export_csv_content();

    assert!(exported.contains("tx_000003,2026-05-05T20:00:00,card_002,250,Suspicious Merchant,electronics,online,CA,US,dev_x,203.0.113.10,false"));
}
