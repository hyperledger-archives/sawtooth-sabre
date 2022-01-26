// Copyright 2018 Cargill Incorporated
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Provides a Sawtooth Transaction Handler for executing Sabre transactions.

use protobuf::Message;
use sawtooth_sdk::messages::processor::TpProcessRequest;
use sawtooth_sdk::processor::handler::ApplyError;
use sawtooth_sdk::processor::handler::TransactionContext;
use sawtooth_sdk::processor::handler::TransactionHandler;

use transact::families::sabre::handler::SabreTransactionHandler;
use transact::handler::ContextError;
use transact::handler::TransactionHandler as TransactHandler;
use transact::protocol::transaction::Transaction;
use transact::protos::transaction::TransactionHeader;

/// The namespace registry prefix for global state (00ec00)
const NAMESPACE_REGISTRY_PREFIX: &str = "00ec00";

/// The contract registry prefix for global state (00ec01)
const CONTRACT_REGISTRY_PREFIX: &str = "00ec01";

/// The contract prefix for global state (00ec02)
const CONTRACT_PREFIX: &str = "00ec02";

struct SabreContext<'a> {
    sawtooth_context: &'a dyn TransactionContext,
}

impl<'a> transact::handler::TransactionContext for SabreContext<'a> {
    fn get_state_entry(&self, address: &str) -> Result<Option<Vec<u8>>, ContextError> {
        let results = self
            .sawtooth_context
            .get_state_entries(&[address.to_owned()])
            .map_err(to_context_error)?;

        // take the first item, if it exists
        Ok(results.into_iter().next().map(|(_, v)| v))
    }

    fn get_state_entries(
        &self,
        addresses: &[String],
    ) -> Result<Vec<(String, Vec<u8>)>, ContextError> {
        self.sawtooth_context
            .get_state_entries(addresses)
            .map_err(to_context_error)
    }

    fn set_state_entry(&self, address: String, data: Vec<u8>) -> Result<(), ContextError> {
        self.set_state_entries(vec![(address, data)])
    }

    fn set_state_entries(&self, entries: Vec<(String, Vec<u8>)>) -> Result<(), ContextError> {
        self.sawtooth_context
            .set_state_entries(entries)
            .map_err(to_context_error)
    }

    fn delete_state_entry(&self, address: &str) -> Result<Option<String>, ContextError> {
        Ok(self
            .delete_state_entries(&[address.to_owned()])?
            .into_iter()
            .next())
    }

    fn delete_state_entries(&self, addresses: &[String]) -> Result<Vec<String>, ContextError> {
        self.sawtooth_context
            .delete_state_entries(addresses)
            .map_err(to_context_error)
    }

    fn add_receipt_data(&self, data: Vec<u8>) -> Result<(), ContextError> {
        self.sawtooth_context
            .add_receipt_data(&data)
            .map_err(to_context_error)
    }

    fn add_event(
        &self,
        event_type: String,
        attributes: Vec<(String, String)>,
        data: Vec<u8>,
    ) -> Result<(), ContextError> {
        self.sawtooth_context
            .add_event(event_type, attributes, &data)
            .map_err(to_context_error)
    }
}

fn to_context_error(err: sawtooth_sdk::processor::handler::ContextError) -> ContextError {
    ContextError::ReceiveError(Box::new(err))
}

pub struct SabreHandler {
    transaction_handler: SabreTransactionHandler,
}

impl SabreHandler {
    pub fn new(transaction_handler: SabreTransactionHandler) -> Self {
        Self {
            transaction_handler,
        }
    }
}

impl TransactionHandler for SabreHandler {
    fn family_name(&self) -> String {
        self.transaction_handler.family_name().to_string()
    }

    fn family_versions(&self) -> Vec<String> {
        self.transaction_handler.family_versions().to_vec()
    }

    fn namespaces(&self) -> Vec<String> {
        vec![
            NAMESPACE_REGISTRY_PREFIX.into(),
            CONTRACT_REGISTRY_PREFIX.into(),
            CONTRACT_PREFIX.into(),
        ]
    }

    fn apply(
        &self,
        request: &TpProcessRequest,
        context: &mut dyn TransactionContext,
    ) -> Result<(), ApplyError> {
        let mut header = TransactionHeader::new();
        header.set_signer_public_key(request.get_header().get_signer_public_key().to_string());

        let header_bytes = header.write_to_bytes().map_err(|_| {
            ApplyError::InvalidTransaction("Unable to convert header to bytes".to_string())
        })?;
        let txn = Transaction::new(
            header_bytes,
            request.get_signature().to_string(),
            request.get_payload().to_vec(),
        );
        let txn_pair = txn
            .into_pair()
            .map_err(|err| ApplyError::InvalidTransaction(err.to_string()))?;

        let mut sabre_context = SabreContext {
            sawtooth_context: context,
        };

        match self
            .transaction_handler
            .apply(&txn_pair, &mut sabre_context)
        {
            Ok(()) => Ok(()),
            Err(transact::handler::ApplyError::InvalidTransaction(msg)) => {
                Err(ApplyError::InvalidTransaction(msg))
            }
            Err(transact::handler::ApplyError::InternalError(msg)) => {
                Err(ApplyError::InternalError(msg))
            }
        }
    }
}
