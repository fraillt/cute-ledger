use std::io::Write;

use crate::processor::ClientId;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Account {
    pub client: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

pub fn print_accounts<W>(
    output: &mut W,
    accounts: impl Iterator<Item = Account>,
) -> anyhow::Result<()>
where
    W: Write,
{
    let mut writer = Writer::from_writer(output);
    for acc in accounts {
        if let Err(err) = writer.serialize(acc) {
            anyhow::bail!("Failed to write to CSV: {err}")
        }
    }
    // Ensure all data is flushed to the output
    if let Err(err) = writer.flush() {
        anyhow::bail!("Failed to flush CSV writer: {err}")
    }
    Ok(())
}
