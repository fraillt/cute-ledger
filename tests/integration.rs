use std::{collections::HashSet, str::from_utf8};

use cute_ledger::bin_utils::Service;

const TEST_FILE: &str = include_str!("transactions.csv");

#[test]
fn process_transactions() {
    let mut output = Vec::new();
    let service = Service {
        input: TEST_FILE.as_bytes(),
        output: &mut output,
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
    service.run().unwrap();
    // since underlying for client accounts container uses cryptographic hash function
    // results are randomized, so we collect lines into hashset
    let lines: HashSet<String> = from_utf8(&output)
        .unwrap()
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(lines.len(), 3);
    assert!(lines.contains("client,available,held,total,locked"));
    assert!(lines.contains("1,1.5,0,1.5,false"));
    assert!(lines.contains("2,2,0,2,false"));
}
