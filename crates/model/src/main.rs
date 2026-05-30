use model::data::transactions::Transactions;

fn main() -> anyhow::Result<()> {
    let transactions = Transactions::parse_csv("transactions.csv")?;
    for transaction in transactions.items {
        println!("{:?}", transaction);
    }
    Ok(())
}
