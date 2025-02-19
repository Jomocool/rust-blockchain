//! # Transactions
//!
//! Accounts send transactions to the blockchain.
//! Within the blockchain, transactions are cryptographically signed.
//! Transactions live within blocks.
use std::sync::Arc;

use crate::account::Account;
use crate::block::BlockNumber;
use crate::bytes::Bytes;
use crate::error::{Result, TypeError};
use eth_trie::{EthTrie, MemoryDB, Trie};
use ethereum_types::{Address, H160, H256, U256, U64};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use utils::crypto::{
    hash, public_key_address, recover_public_key, sign_recovery, verify, Signature,
};
use utils::{PublicKey, RecoverableSignature, RecoveryId, SecretKey};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
pub struct Transaction {
    pub from: Address,
    pub to: Option<Address>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<H256>,
    pub nonce: Option<U256>,
    pub value: U256,
    pub data: Option<Bytes>,
    pub gas: U256,
    pub gas_price: U256,
}

pub enum TransactionKind {
    Regular(Address, Address, U256), // a transaction from one account to another.
    ContractDeployment(Address, Bytes), // a transaction without a 'to' address, where the data field is used for the contract code.
    ContractExecution(Address, Address, Bytes), // a transaction that interacts with a deployed smart contract. In this case, 'to' address is the smart contract address.
}

impl Transaction {
    pub fn new(
        from: Account,
        to: Option<Account>,
        value: U256,
        nonce: Option<U256>,
        data: Option<Bytes>,
    ) -> Result<Self> {
        let mut transaction = Self {
            from,
            to,
            value,
            nonce,
            hash: None,
            data,
            gas: U256::from(10),
            gas_price: U256::from(10),
        };

        transaction.hash()?;

        Ok(transaction)
    }

    pub fn hash(&mut self) -> Result<H256> {
        let serialized = bincode::serialize(&self)?;
        let hash: H256 = hash(&serialized).into();
        self.hash = Some(hash);

        self.transaction_hash()
    }

    pub fn transaction_hash(&self) -> Result<H256> {
        self.hash.ok_or(TypeError::MissingTransactionHash)
    }

    pub fn kind(self) -> Result<TransactionKind> {
        match (self.from, self.to, self.data) {
            (from, Some(to), None) => Ok(TransactionKind::Regular(from, to, self.value)),
            (from, None, Some(data)) => Ok(TransactionKind::ContractDeployment(from, data)),
            (from, Some(to), Some(data)) => Ok(TransactionKind::ContractExecution(from, to, data)),
            _ => Err(TypeError::InvalidTransaction("kind".into())),
        }
    }

    pub fn sign(&self, key: SecretKey) -> Result<SignedTransaction> {
        let encoded = bincode::serialize(&self)?;
        let recoverable_signature = sign_recovery(&encoded, &key)?;
        let (_, signature_bytes) = recoverable_signature.serialize_compact();
        let Signature { v, r, s } = recoverable_signature.into();
        let transaction_hash = hash(&signature_bytes).into();

        let signed_transaction = SignedTransaction {
            v,
            r,
            s,
            raw_transaction: encoded.into(),
            transaction_hash,
        };

        Ok(signed_transaction)
    }

    pub fn verify(signed_transaction: SignedTransaction, address: Address) -> Result<bool> {
        let (message, recovery_id, signature_bytes) = Self::recover_pieces(signed_transaction)?;
        let key = recover_public_key(&message, &signature_bytes, recovery_id.to_i32())?;
        let verified = verify(&message, &signature_bytes, &key)?;
        let addresses_match = address == public_key_address(&key);

        Ok(verified && addresses_match)
    }

    pub fn recover_address(signed_transaction: SignedTransaction) -> Result<H160> {
        let key = Self::recover_public_key(signed_transaction)?;
        let address = public_key_address(&key);

        Ok(address)
    }

    pub fn recover_public_key(signed_transaction: SignedTransaction) -> Result<PublicKey> {
        let (message, recovery_id, signature_bytes) = Self::recover_pieces(signed_transaction)?;
        let key = recover_public_key(&message, &signature_bytes, recovery_id.to_i32())?;

        Ok(key)
    }

    fn recover_pieces(
        signed_transaction: SignedTransaction,
    ) -> Result<(Vec<u8>, RecoveryId, [u8; 64])> {
        let message = signed_transaction.raw_transaction.to_owned();
        let signature: Signature = signed_transaction.into();
        let recoverable_signature: RecoverableSignature = signature.try_into()?;
        let (recovery_id, signature_bytes) = recoverable_signature.serialize_compact();

        Ok((message.to_vec(), recovery_id, signature_bytes))
    }

    fn to_trie(transactions: &[Transaction]) -> Result<EthTrie<MemoryDB>> {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut trie = EthTrie::new(memdb);

        transactions.iter().try_for_each(|transaction| {
            trie.insert(
                transaction.transaction_hash()?.as_bytes(),
                bincode::serialize(&transaction)?.as_slice(),
            )
            .map_err(|e| TypeError::TrieError(format!("Error inserting transactions: {}", e)))
        })?;

        Ok(trie)
    }

    pub fn root_hash(transactions: &[Transaction]) -> Result<H256> {
        let mut trie = Self::to_trie(transactions)?;
        let root_hash = trie
            .root_hash()
            .map_err(|e| TypeError::TrieError(format!("Error calculating root hash: {}", e)))?;

        Ok(H256::from_slice(root_hash.as_bytes()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedTransaction {
    pub v: u64,
    pub r: H256,
    pub s: H256,
    pub raw_transaction: Bytes,
    pub transaction_hash: H256,
}

impl From<SignedTransaction> for Signature {
    fn from(value: SignedTransaction) -> Self {
        Signature {
            v: value.v,
            r: value.r,
            s: value.s,
        }
    }
}

impl TryInto<Transaction> for SignedTransaction {
    type Error = TypeError;

    fn try_into(self) -> Result<Transaction> {
        bincode::deserialize(&self.raw_transaction)
            .map_err(|e| TypeError::EncodingDecodingError(e.to_string()))
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
pub struct TransactionRequest {
    pub data: Option<Bytes>,
    pub gas: U256,
    pub gas_price: U256,
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub value: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<U256>,
}

impl From<Transaction> for TransactionRequest {
    fn from(value: Transaction) -> TransactionRequest {
        TransactionRequest {
            from: Some(value.from),
            to: value.to,
            value: Some(value.value),
            data: value.data,
            gas: value.gas,
            gas_price: value.gas_price,
            nonce: value.nonce,
            r: None,
            s: None,
        }
    }
}

impl TryInto<Transaction> for TransactionRequest {
    type Error = TypeError;

    fn try_into(self) -> Result<Transaction> {
        let value = self.value.unwrap_or(U256::zero());
        let from = self.from.unwrap_or(H160::zero());
        Transaction::new(from, self.to, value, self.nonce, self.data)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
pub struct TransactionReceipt {
    pub block_hash: Option<H256>,
    pub block_number: Option<BlockNumber>,
    pub contract_address: Option<H160>,
    pub transaction_hash: H256,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct Log {
    pub address: H160,
    pub block_hash: Option<H256>,
    pub block_number: Option<U64>,
    pub data: Bytes,
    pub log_index: Option<U256>,
    pub log_type: Option<String>,
    pub removed: Option<bool>,
    pub topics: Vec<H256>,
    pub transaction_hash: Option<H256>,
    pub transaction_index: Option<String>,
    pub transaction_log_index: Option<U256>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_types::U256;
    use std::{convert::From, str::FromStr};
    use utils::crypto::{keypair, public_key_address};

    pub(crate) fn new_transaction() -> Transaction {
        let from = H160::from_str("0x4a0d457e884ebd9b9773d172ed687417caac4f14").unwrap();
        let to = H160::from_str("0x6b78fa07883d5c5b527da9828ac77f5aa5a61d3b").unwrap();
        let value = U256::from(1u64);

        Transaction::new(from, Some(to), value, None, None).unwrap()
    }

    #[test]
    fn it_recovers_an_address_from_a_signed_transaction() {
        let (secret_key, public_key) = keypair();
        let transaction = new_transaction();
        let signed = transaction.sign(secret_key).unwrap();
        let recovered = Transaction::recover_address(signed).unwrap();

        assert_eq!(recovered, public_key_address(&public_key));
    }

    #[test]
    fn it_verifies_a_signed_transaction() {
        let (secret_key, public_key) = keypair();
        let mut transaction = new_transaction();
        transaction.from = public_key_address(&public_key);
        let signed = transaction.sign(secret_key).unwrap();
        let verifies = Transaction::verify(signed, transaction.from).unwrap();

        assert!(verifies);
    }

    #[test]
    fn root_hash() {
        let transaction_1 = new_transaction();
        let transaction_2 = new_transaction();
        let root = Transaction::root_hash(&vec![transaction_1, transaction_2]).unwrap();
        let expected =
            H256::from_str("0xa3b8c35bab6501806ed681220afe26a0d46774a6aa56d044b0f6aef0f3f0d682")
                .unwrap();
        assert_eq!(root, expected);
    }
}
