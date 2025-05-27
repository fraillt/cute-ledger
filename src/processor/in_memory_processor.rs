use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::{
    account::{Account, TransactionId},
    command::{AccountCommand, CreateTransactionCommand, TransactionKind},
};

use super::{ClientId, TransactionProcessError, TransactionProcessor};

#[derive(Default)]
pub struct InMemoryTransactionProcessor {
    created_tx_list: HashMap<TransactionId, CreateTransactionCommand>,
    pub accounts: HashMap<ClientId, Account>,
}

impl TransactionProcessor for InMemoryTransactionProcessor {
    fn process_transaction(
        &mut self,
        tx_id: TransactionId,
        client_id: ClientId,
        amount: Option<Decimal>,
        kind: TransactionKind,
    ) -> Result<(), TransactionProcessError> {
        let tx_entry = self.created_tx_list.entry(tx_id);
        let cmd = AccountCommand::parse_command(&tx_entry, kind, amount)?;
        let acc = self.accounts.entry(client_id).or_default();
        match cmd {
            AccountCommand::CreateTx(command) => {
                let evt = acc.handle_new_transaction(command.clone())?;
                acc.apply(&evt);
                // insert only when command succeeded
                tx_entry.insert_entry(command);
            }
            AccountCommand::ModifyTx(command) => {
                let evt = acc.handle_modify_transaction(command)?;
                acc.apply(&evt);
            }
        };
        Ok(())
    }
}
