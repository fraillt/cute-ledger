use std::{fs::File, io::Read};

use anyhow::{Context, Result};
use csv::{DeserializeRecordsIntoIter, Trim};
use cute_ledger::{
    command::TransactionKind,
    processor::{TransactionProcessor, in_memory_processor::InMemoryTransactionProcessor},
};
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub kind: TransactionKind,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

/// Parses transaction list in CSV format
///
/// # Panics
///
/// If transaction cannot be parsed
pub struct CsvTransactionParser<R> {
    iter: DeserializeRecordsIntoIter<R, Transaction>,
}

impl<R> CsvTransactionParser<R>
where
    R: Read,
{
    pub fn new(source: R) -> Self {
        let reader = csv::ReaderBuilder::new()
            .trim(Trim::All)
            .flexible(true)
            .from_reader(source);

        Self {
            iter: reader.into_deserialize(),
        }
    }
}

impl<R> Iterator for CsvTransactionParser<R>
where
    R: Read,
{
    type Item = (u64, Transaction);

    fn next(&mut self) -> Option<Self::Item> {
        let curr_line = self.iter.reader().position().line();
        self.iter.next().map(|row| (curr_line, row.unwrap()))
    }
}

fn main() -> Result<()> {
    let filename = std::env::args()
        .nth(1)
        .context("Expected a file name as the first argument")?;
    let file = File::open(&filename).with_context(|| format!("Failed to open `{filename}`"))?;

    let parser = CsvTransactionParser::new(file);

    let mut processor = InMemoryTransactionProcessor::default();

    for (line, row) in parser {
        if let Err(err) = processor.process_transaction(row.tx, row.client, row.amount, row.kind) {
            eprintln!("Error at line {line}: {err}")
        }
    }

    Ok(())
}
