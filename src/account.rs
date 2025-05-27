use std::collections::HashSet;

use rust_decimal::Decimal;
use thiserror::Error;

use crate::command::{
    CreateTransactionAction, CreateTransactionCommand, ModifyTransactionAction,
    ModifyTransactionCommand,
};

pub type TransactionId = u32;

#[derive(Debug, PartialEq, Eq)]
pub enum AccountEventKind {
    Deposited,
    Withdrawn,
    Disputed,
    Resolved,
    Chargedback,
}

#[derive(Debug)]
pub struct AccountEvent {
    transaction_id: TransactionId,
    amount: Decimal,
    kind: AccountEventKind,
}

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("Account is frozen, no further operations are allowed")]
    AccountFrozen,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("{action:?} cannot be initiated, because the transaction is {dispute_state_str}")]
    TransactionDisputeStateMismatch {
        action: ModifyTransactionAction,
        dispute_state_str: String,
    },
    #[error("Dispute operation is not supported for parent transaction")]
    DisputeNotSupported,
}

#[derive(Debug, Default)]
pub struct Account {
    available: Decimal,
    held: Decimal,
    locked: bool,
    txs_under_dispute: HashSet<TransactionId>,
}

impl Account {
    pub fn total_amount(&self) -> Decimal {
        self.available + self.held
    }

    pub fn apply(&mut self, event: &AccountEvent) {
        match event.kind {
            AccountEventKind::Deposited => {
                self.available += event.amount;
            }
            AccountEventKind::Withdrawn => {
                self.available -= event.amount;
            }
            AccountEventKind::Disputed => {
                self.available -= event.amount;
                self.held += event.amount;
                self.txs_under_dispute.insert(event.transaction_id);
            }
            AccountEventKind::Resolved => {
                self.available += event.amount;
                self.held -= event.amount;
                self.txs_under_dispute.remove(&event.transaction_id);
            }
            AccountEventKind::Chargedback => {
                self.held -= event.amount;
                self.locked = true;
                self.txs_under_dispute.remove(&event.transaction_id);
            }
        }
    }

    pub fn handle_new_transaction(
        &self,
        command: CreateTransactionCommand,
    ) -> Result<AccountEvent, AccountError> {
        if self.locked {
            return Err(AccountError::AccountFrozen);
        }

        match command.action {
            CreateTransactionAction::Deposit => Ok(AccountEvent {
                transaction_id: command.tx_id,
                amount: command.amount,
                kind: AccountEventKind::Deposited,
            }),
            CreateTransactionAction::Withdraw => {
                if self.available >= command.amount {
                    Ok(AccountEvent {
                        transaction_id: command.tx_id,
                        amount: command.amount,
                        kind: AccountEventKind::Withdrawn,
                    })
                } else {
                    Err(AccountError::InsufficientFunds)
                }
            }
        }
    }

    pub fn handle_modify_transaction(
        &self,
        command: ModifyTransactionCommand,
    ) -> Result<AccountEvent, AccountError> {
        if self.locked {
            return Err(AccountError::AccountFrozen);
        }
        let amount = command.amount;
        let transaction_id = command.tx_id;

        let under_dispute = self.txs_under_dispute.contains(&command.tx_id);

        match (command.action, under_dispute) {
            (ModifyTransactionAction::Dispute, false) => {
                match command.create_action {
                    CreateTransactionAction::Deposit => {
                        // Question: maybe it makes sense to check available balance?
                        Ok(AccountEvent {
                            transaction_id,
                            amount,
                            kind: AccountEventKind::Disputed,
                        })
                    }
                    CreateTransactionAction::Withdraw => Err(AccountError::DisputeNotSupported),
                }
            }
            (ModifyTransactionAction::Resolve, true) => Ok(AccountEvent {
                transaction_id,
                amount,
                kind: AccountEventKind::Resolved,
            }),
            (ModifyTransactionAction::Chargeback, true) => Ok(AccountEvent {
                transaction_id,
                amount,
                kind: AccountEventKind::Chargedback,
            }),
            _ => Err(AccountError::TransactionDisputeStateMismatch {
                action: command.action,
                dispute_state_str: if under_dispute {
                    "already under dispute".to_string()
                } else {
                    "not under dispute".to_string()
                },
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::prelude::{FromPrimitive, Zero};

    use super::*;

    #[test]
    fn apply_events() {
        let mut acc = Account::default();
        acc.apply(&AccountEvent {
            transaction_id: 0,
            amount: Decimal::from_u32(10).unwrap(),
            kind: AccountEventKind::Deposited,
        });
        assert_eq!(acc.available, Decimal::from_u32(10).unwrap());
        assert_eq!(acc.held, Decimal::zero());
        assert!(acc.txs_under_dispute.is_empty());
        acc.apply(&AccountEvent {
            transaction_id: 1,
            amount: Decimal::from_u32(3).unwrap(),
            kind: AccountEventKind::Withdrawn,
        });
        assert_eq!(acc.available, Decimal::from_u32(7).unwrap());
        assert_eq!(acc.held, Decimal::zero());
        assert!(acc.txs_under_dispute.is_empty());
        // event is the source of truth, there's no more validation happening
        acc.apply(&AccountEvent {
            transaction_id: 3,
            amount: Decimal::from_u32(5).unwrap(),
            kind: AccountEventKind::Disputed,
        });
        assert_eq!(acc.available, Decimal::from_u32(2).unwrap());
        assert_eq!(acc.held, Decimal::from_u32(5).unwrap());
        assert_eq!(acc.txs_under_dispute.len(), 1);
        acc.apply(&AccountEvent {
            transaction_id: 3,
            amount: Decimal::from_u32(5).unwrap(),
            kind: AccountEventKind::Resolved,
        });
        assert_eq!(acc.available, Decimal::from_u32(7).unwrap());
        assert_eq!(acc.held, Decimal::from_u32(0).unwrap());
        assert!(acc.txs_under_dispute.is_empty());
        assert!(!acc.locked);

        acc.apply(&AccountEvent {
            transaction_id: 5,
            amount: Decimal::from_u32(5).unwrap(),
            kind: AccountEventKind::Disputed,
        });
        acc.apply(&AccountEvent {
            transaction_id: 5,
            amount: Decimal::from_u32(5).unwrap(),
            kind: AccountEventKind::Chargedback,
        });
        assert_eq!(acc.available, Decimal::from_u32(2).unwrap());
        assert_eq!(acc.held, Decimal::from_u32(0).unwrap());
        assert!(acc.txs_under_dispute.is_empty());
        assert!(acc.locked)
    }

    #[test]
    fn verify_total_amount() {
        let acc = Account {
            available: Decimal::from_u32(10).unwrap(),
            held: Decimal::from_u32(3).unwrap(),
            ..Default::default()
        };
        assert_eq!(acc.total_amount(), Decimal::from_u32(13).unwrap());
    }

    #[test]
    fn handle_new_transaction() {
        let mut acc = Account::default();

        // deposit
        let deposit_evt = acc
            .handle_new_transaction(CreateTransactionCommand {
                tx_id: 0,
                action: CreateTransactionAction::Deposit,
                amount: Decimal::from_u32(13).unwrap(),
            })
            .unwrap();
        assert_eq!(deposit_evt.amount, Decimal::from_u32(13).unwrap());
        assert!(matches!(deposit_evt.kind, AccountEventKind::Deposited));

        // withdrawal
        let withdrawal_cmd = CreateTransactionCommand {
            tx_id: 0,
            action: CreateTransactionAction::Withdraw,
            amount: Decimal::from_u32(5).unwrap(),
        };
        let err = acc
            .handle_new_transaction(withdrawal_cmd.clone())
            .unwrap_err();
        assert!(matches!(err, AccountError::InsufficientFunds));

        // withdrawal after deposit applied
        acc.apply(&deposit_evt);
        let withdrawal_evt = acc.handle_new_transaction(withdrawal_cmd.clone()).unwrap();
        assert_eq!(withdrawal_evt.amount, Decimal::from_u32(5).unwrap());
        assert!(matches!(withdrawal_evt.kind, AccountEventKind::Withdrawn));

        // account locked
        acc.locked = true;
        let err = acc.handle_new_transaction(withdrawal_cmd).unwrap_err();
        assert!(matches!(err, AccountError::AccountFrozen));
    }

    #[test]
    fn handle_modify_transaction() {
        let mut acc = Account::default();
        let deposit_evt = AccountEvent {
            transaction_id: 1,
            amount: Decimal::from_u32(13).unwrap(),
            kind: AccountEventKind::Deposited,
        };
        acc.apply(&deposit_evt);

        // dispute
        let dispute_cmd = ModifyTransactionCommand {
            tx_id: 1,
            action: ModifyTransactionAction::Dispute,
            amount: Decimal::from_u32(13).unwrap(),
            create_action: CreateTransactionAction::Deposit,
        };
        let dispute_evt = acc.handle_modify_transaction(dispute_cmd.clone()).unwrap();
        assert_eq!(dispute_evt.amount, Decimal::from_u32(13).unwrap());
        assert!(matches!(dispute_evt.kind, AccountEventKind::Disputed));

        // dispute for withdrawal not supported
        let err = acc
            .handle_modify_transaction(ModifyTransactionCommand {
                create_action: CreateTransactionAction::Withdraw,
                ..dispute_cmd
            })
            .unwrap_err();
        assert!(matches!(&err, AccountError::DisputeNotSupported));

        // dispute twice not allowed
        acc.apply(&dispute_evt);
        let err = acc
            .handle_modify_transaction(dispute_cmd.clone())
            .unwrap_err();
        assert!(matches!(
            &err,
            AccountError::TransactionDisputeStateMismatch {
                action: ModifyTransactionAction::Dispute,
                dispute_state_str: _
            }
        ));
        assert_eq!(
            err.to_string(),
            "Dispute cannot be initiated, because the transaction is already under dispute"
        );

        // resolve transaction
        let resolve_cmd = ModifyTransactionCommand {
            tx_id: 1,
            action: ModifyTransactionAction::Resolve,
            amount: Decimal::from_u32(13).unwrap(),
            create_action: CreateTransactionAction::Deposit,
        };
        let resolve_evt = acc.handle_modify_transaction(resolve_cmd.clone()).unwrap();
        assert_eq!(resolve_evt.amount, Decimal::from_u32(13).unwrap());
        assert_eq!(resolve_evt.kind, AccountEventKind::Resolved);

        // cannot resolve already resolved
        acc.apply(&resolve_evt);
        let err = acc.handle_modify_transaction(resolve_cmd).unwrap_err();
        assert!(matches!(
            &err,
            AccountError::TransactionDisputeStateMismatch {
                action: ModifyTransactionAction::Resolve,
                dispute_state_str: _
            }
        ));
        assert_eq!(
            err.to_string(),
            format!("Resolve cannot be initiated, because the transaction is not under dispute")
        );

        // chargeback transaction
        acc.apply(&dispute_evt);
        let chargeback_cmd = ModifyTransactionCommand {
            tx_id: 1,
            action: ModifyTransactionAction::Chargeback,
            amount: Decimal::from_u32(13).unwrap(),
            create_action: CreateTransactionAction::Deposit,
        };
        let chargeback_evt = acc
            .handle_modify_transaction(chargeback_cmd.clone())
            .unwrap();
        assert_eq!(chargeback_evt.amount, Decimal::from_u32(13).unwrap());
        assert_eq!(chargeback_evt.kind, AccountEventKind::Chargedback);

        // any further command returns error
        acc.apply(&chargeback_evt);
        let err = acc
            .handle_modify_transaction(dispute_cmd.clone())
            .unwrap_err();
        assert!(matches!(err, AccountError::AccountFrozen));
    }
}
