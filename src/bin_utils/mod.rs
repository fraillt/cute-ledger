//! This module could be a separate crate on its own, to bootstrap [`cute_ledger`] within binary
//! but for simplicitly purposes, I include this module directly in binary.

use std::io::{Read, Write};

use crate::processor::{
    TransactionProcessError, TransactionProcessor,
    in_memory_processor::InMemoryTransactionProcessor,
};
use anyhow::Result;
use csv_parser::CsvTransactionParser;
use csv_printer::{Account, print_accounts};
pub mod csv_parser;
pub mod csv_printer;

pub struct Service<'w, R, W: 'w> {
    pub input: R,
    pub output: &'w mut W,
    pub error_printer: Box<dyn FnMut(u64, TransactionProcessError)>,
}

impl<'w, R, W> Service<'w, R, W>
where
    R: Read,
    W: Write + 'w,
{
    pub fn run(mut self) -> Result<()> {
        let parser = CsvTransactionParser::new(self.input);

        let mut processor = InMemoryTransactionProcessor::default();

        for (line, row) in parser {
            if let Err(err) =
                processor.process_transaction(row.tx, row.client, row.amount, row.kind)
            {
                (self.error_printer)(line, err);
            }
        }

        print_accounts(
            self.output,
            processor.accounts.iter().map(|(client_id, acc)| Account {
                client: *client_id,
                available: acc.available(),
                held: acc.held(),
                locked: acc.locked(),
                total: acc.total_amount(),
            }),
        )
    }
}
