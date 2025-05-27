use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    account::{AccountError, TransactionId},
    command::{AccountCommandError, TransactionKind},
};

pub mod in_memory_processor;

#[derive(Debug, Error)]
pub enum TransactionProcessError {
    #[error(transparent)]
    CommandErr(#[from] AccountCommandError),
    #[error(transparent)]
    AccountErr(#[from] AccountError),
}

pub type ClientId = u16;

pub trait TransactionProcessor {
    fn process_transaction(
        &mut self,
        tx_id: TransactionId,
        client_id: ClientId,
        amount: Option<Decimal>,
        kind: TransactionKind,
    ) -> Result<(), TransactionProcessError>;
}
