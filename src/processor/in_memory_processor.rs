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
                let evt = acc.handle_create_transaction(command.clone())?;
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

#[cfg(test)]
mod tests {

    use rust_decimal::prelude::FromPrimitive;

    use crate::command::{AccountCommandError, ModifyTransactionAction};

    use super::*;

    #[test]
    fn process_some_transactions() {
        let mut processor = InMemoryTransactionProcessor::default();
        processor
            .process_transaction(
                1,
                1,
                Some(Decimal::from_u32(10).unwrap()),
                TransactionKind::Deposit,
            )
            .unwrap();
        processor
            .process_transaction(
                2,
                2,
                Some(Decimal::from_u32(10).unwrap()),
                TransactionKind::Deposit,
            )
            .unwrap();
        assert_eq!(processor.accounts.len(), 2);
        assert_eq!(processor.created_tx_list.len(), 2);

        processor
            .process_transaction(
                2,
                2,
                Some(Decimal::from_u32(10).unwrap()),
                TransactionKind::Dispute,
            )
            .unwrap();
        assert_eq!(processor.accounts.len(), 2);
        assert_eq!(processor.created_tx_list.len(), 2);

        let a1 = processor.accounts.get(&1).unwrap();
        assert_eq!(a1.available(), Decimal::from_u32(10).unwrap());
        assert_eq!(a1.held(), Decimal::from_u32(0).unwrap());

        let a2 = processor.accounts.get(&2).unwrap();
        assert_eq!(a2.available(), Decimal::from_u32(0).unwrap());
        assert_eq!(a2.held(), Decimal::from_u32(10).unwrap());

        let err = processor
            .process_transaction(
                3,
                2,
                Some(Decimal::from_u32(10).unwrap()),
                TransactionKind::Dispute,
            )
            .unwrap_err();

        assert!(matches!(
            err,
            TransactionProcessError::CommandErr(AccountCommandError::ExistingTxRequired {
                action: ModifyTransactionAction::Dispute
            })
        ))
    }
}
