use std::fs::File;

use anyhow::{Context, Result};
use cute_ledger::bin_utils::Service;

fn main() -> Result<()> {
    let filename = std::env::args()
        .nth(1)
        .context("Expected a file name as the first argument")?;
    let file = File::open(&filename).with_context(|| format!("Failed to open `{filename}`"))?;

    let service = Service {
        input: file,
        output: &mut std::io::stdout(),
        error_printer: Box::new(|line, err| {
            match err {
                cute_ledger::processor::TransactionProcessError::CommandErr(err) => {
                    eprintln!("Error at line {line}: {err}")
                }
                cute_ledger::processor::TransactionProcessError::AccountErr(_) => {
                    // these are not technical errors, so we don't need to print them
                }
            }
        }),
    };
    service.run()
}
