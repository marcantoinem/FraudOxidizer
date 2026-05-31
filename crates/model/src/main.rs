use model::data::transactions::Transactions;

fn main() -> anyhow::Result<()> {
    let mut transactions = Transactions::parse_csv("transactions.csv")?;
    transactions.apply_fraud_factors();

    let fraud_transactions: Vec<_> = transactions
        .items
        .iter()
        .filter(|transaction| transaction.likely_fraud())
        .collect();

    println!(
        "Detected {} likely fraud transactions out of {} rows.",
        fraud_transactions.len(),
        transactions.items.len()
    );

    for transaction in fraud_transactions {
        println!(
            "- {} card {} score {:.2} country {}",
            transaction.transaction_id.0,
            transaction.card_id.0,
            transaction.fraud_score(),
            transaction.cardholder_country.0.alpha2(),
        );

        for factor in &transaction.fraud_factors {
            println!(
                "  reason: {} (weight {:.2})",
                factor.reason(),
                factor.weight()
            );
        }
    }

    Ok(())
}
