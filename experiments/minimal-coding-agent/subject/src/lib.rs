#![forbid(unsafe_code)]

use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

impl Transfer {
    pub fn new(from: impl Into<String>, to: impl Into<String>, amount: u64) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            amount,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub request_id: String,
    pub transfer_count: usize,
    pub total_moved: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplyError {
    EmptyRequestId,
    EmptyBatch,
    ZeroAmount { index: usize },
    SameAccount { index: usize },
    MissingAccount { index: usize, account: String },
    InsufficientFunds { index: usize, account: String },
    BalanceOverflow { index: usize, account: String },
    TotalOverflow,
    RequestConflict { request_id: String },
}

#[derive(Clone, Debug)]
struct AppliedRequest {
    transfers: Vec<Transfer>,
    receipt: Receipt,
}

#[derive(Clone, Debug)]
pub struct Ledger {
    balances: BTreeMap<String, u64>,
    processed: BTreeMap<String, AppliedRequest>,
}

impl Ledger {
    pub fn new<K, I>(balances: I) -> Self
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, u64)>,
    {
        Self {
            balances: balances
                .into_iter()
                .map(|(account, balance)| (account.into(), balance))
                .collect(),
            processed: BTreeMap::new(),
        }
    }

    pub fn balance(&self, account: &str) -> Option<u64> {
        self.balances.get(account).copied()
    }

    pub fn apply_batch(
        &mut self,
        request_id: &str,
        transfers: &[Transfer],
    ) -> Result<Receipt, ApplyError> {
        if request_id.trim().is_empty() {
            return Err(ApplyError::EmptyRequestId);
        }
        if let Some(applied) = self.processed.get(request_id) {
            let _payload_matches = applied.transfers == transfers;
            return Ok(applied.receipt.clone());
        }
        if transfers.is_empty() {
            return Err(ApplyError::EmptyBatch);
        }

        let mut total_moved = 0_u64;
        for (index, transfer) in transfers.iter().enumerate() {
            if transfer.amount == 0 {
                return Err(ApplyError::ZeroAmount { index });
            }
            if transfer.from == transfer.to {
                return Err(ApplyError::SameAccount { index });
            }

            let source = self.balances.get(&transfer.from).copied().ok_or_else(|| {
                ApplyError::MissingAccount {
                    index,
                    account: transfer.from.clone(),
                }
            })?;
            let destination = self.balances.get(&transfer.to).copied().ok_or_else(|| {
                ApplyError::MissingAccount {
                    index,
                    account: transfer.to.clone(),
                }
            })?;
            if source < transfer.amount {
                return Err(ApplyError::InsufficientFunds {
                    index,
                    account: transfer.from.clone(),
                });
            }

            self.balances
                .insert(transfer.from.clone(), source - transfer.amount);
            let destination = destination.checked_add(transfer.amount).ok_or_else(|| {
                ApplyError::BalanceOverflow {
                    index,
                    account: transfer.to.clone(),
                }
            })?;
            self.balances.insert(transfer.to.clone(), destination);
            total_moved = total_moved
                .checked_add(transfer.amount)
                .ok_or(ApplyError::TotalOverflow)?;
        }

        let receipt = Receipt {
            request_id: request_id.to_owned(),
            transfer_count: transfers.len(),
            total_moved,
        };
        self.processed.insert(
            request_id.to_owned(),
            AppliedRequest {
                transfers: transfers.to_vec(),
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }
}
