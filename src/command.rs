use std::collections::hash_map::Entry;

use rust_decimal::{Decimal, prelude::Zero};
use serde::Deserialize;
use thiserror::Error;

use crate::account::TransactionId;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, Copy)]
pub enum CreateTransactionAction {
    Deposit,
    Withdraw,
}

#[derive(Debug, Clone, Copy)]
pub enum ModifyTransactionAction {
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone)]
pub struct CreateTransactionCommand {
    pub tx_id: TransactionId,
    pub action: CreateTransactionAction,
    pub amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct ModifyTransactionCommand {
    pub tx_id: TransactionId,
    pub action: ModifyTransactionAction,
    pub amount: Decimal,
    pub create_action: CreateTransactionAction,
}

#[derive(Debug, Error)]
pub enum AccountCommandError {
    #[error("Amount is required for {action:?}")]
    AmountRequired { action: CreateTransactionAction },
    #[error("Amount must not be negative for {action:?}")]
    NegativeAmount { action: CreateTransactionAction },
    #[error("There should be an existing transaction for {action:?}")]
    ExistingTxRequired { action: ModifyTransactionAction },
    #[error("There shouldn't be an existing transaction for {action:?}")]
    DuplicateTransaction { action: CreateTransactionAction },
}

pub enum AccountCommand {
    CreateTx(CreateTransactionCommand),
    ModifyTx(ModifyTransactionCommand),
}

impl AccountCommand {
    pub fn parse_command(
        tx_entry: &Entry<'_, TransactionId, CreateTransactionCommand>,
        kind: TransactionKind,
        amount: Option<Decimal>,
    ) -> Result<Self, AccountCommandError> {
        match kind {
            TransactionKind::Deposit => Ok(Self::CreateTx(Self::parse_create_command(
                tx_entry,
                amount,
                CreateTransactionAction::Deposit,
            )?)),
            TransactionKind::Withdrawal => Ok(Self::CreateTx(Self::parse_create_command(
                tx_entry,
                amount,
                CreateTransactionAction::Withdraw,
            )?)),
            TransactionKind::Dispute => Ok(Self::ModifyTx(Self::parse_modify_command(
                tx_entry,
                ModifyTransactionAction::Dispute,
            )?)),
            TransactionKind::Resolve => Ok(Self::ModifyTx(Self::parse_modify_command(
                tx_entry,
                ModifyTransactionAction::Resolve,
            )?)),
            TransactionKind::Chargeback => Ok(Self::ModifyTx(Self::parse_modify_command(
                tx_entry,
                ModifyTransactionAction::Chargeback,
            )?)),
        }
    }

    fn parse_create_command(
        entry: &Entry<'_, TransactionId, CreateTransactionCommand>,
        amount: Option<Decimal>,
        action: CreateTransactionAction,
    ) -> Result<CreateTransactionCommand, AccountCommandError> {
        let Entry::Vacant(entry) = entry else {
            return Err(AccountCommandError::DuplicateTransaction { action });
        };
        if let Some(amount) = amount {
            if amount >= Decimal::zero() {
                Ok(CreateTransactionCommand {
                    tx_id: *entry.key(),
                    action,
                    amount,
                })
            } else {
                Err(AccountCommandError::NegativeAmount { action })
            }
        } else {
            Err(AccountCommandError::AmountRequired { action })
        }
    }

    fn parse_modify_command(
        tx_entry: &Entry<'_, TransactionId, CreateTransactionCommand>,
        action: ModifyTransactionAction,
    ) -> Result<ModifyTransactionCommand, AccountCommandError> {
        let Entry::Occupied(entry) = tx_entry else {
            return Err(AccountCommandError::ExistingTxRequired { action });
        };
        Ok(ModifyTransactionCommand {
            tx_id: *entry.key(),
            action,
            amount: entry.get().amount,
            create_action: entry.get().action,
        })
    }
}
