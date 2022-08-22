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

use crypto::digest::Digest;
use crypto::sha2::Sha512;
use hex::{decode, encode_upper};

use std::collections::BTreeMap;
use std::collections::HashMap;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use sabre_sdk::ApplyError;
        use sabre_sdk::TransactionContext;
        use sabre_sdk::TransactionHandler;
        use sabre_sdk::TpProcessRequest;
        use sabre_sdk::{WasmPtr, execute_entrypoint};
    } else {
        use sawtooth_sdk::processor::handler::ApplyError;
        use sawtooth_sdk::processor::handler::TransactionContext;
        use sawtooth_sdk::processor::handler::TransactionHandler;
        use sawtooth_sdk::messages::processor::TpProcessRequest;
    }
}

const MAX_VALUE: u32 = 4_294_967_295;
const MAX_NAME_LEN: usize = 20;

fn get_intkey_prefix() -> String {
    let mut sha = Sha512::new();
    sha.input_str("intkey");
    sha.result_str()[..6].to_string()
}

fn decode_intkey(hex_string: String) -> Result<BTreeMap<String, u32>, ApplyError> {
    let mut output: BTreeMap<String, u32> = BTreeMap::new();

    // First two characters should be A followed by the number of elements.
    // Only check for A as this will be a map with 15 or less elements
    // It is unlikely that an address will have that many hash collisions.
    let data_type = hex_string
        .get(..1)
        .ok_or_else(|| ApplyError::InvalidTransaction("Unable to get data type".into()))?;
    if data_type != "A" {
        return Err(ApplyError::InvalidTransaction(String::from(
            "Cbor is not a map.",
        )));
    };

    let entries_hex = hex_string.get(1..2).ok_or_else(|| {
        ApplyError::InvalidTransaction("Unable to get number of entries in the map".into())
    })?;

    let entries = u32::from_str_radix(entries_hex, 16)
        .map_err(|err| ApplyError::InvalidTransaction(format!("Unable to decode cbor: {}", err)))?;

    let mut start = 2;

    // For each entry get the Name and Value
    for _n in 0..entries {
        let string_hex = hex_string.get(start..start + 2).ok_or_else(|| {
            ApplyError::InvalidTransaction("Unable to hex for the string data".into())
        })?;

        let string_type = usize::from_str_radix(string_hex, 16).map_err(|err| {
            ApplyError::InvalidTransaction(format!("Unable to decode cbor: {}", err))
        })?;

        // String starts at hex 60 plus the length of the string.
        // For Names it should range from hex 61 (decimal 97) to 74 (decimal 116) because a name
        // cannot be empty and must not be greater than 20 characters
        if !(97..=116).contains(&string_type) {
            return Err(ApplyError::InvalidTransaction(String::from(
                "Name is either too long, too short, or not a string.",
            )));
        }
        start += 2;
        let length = (string_type - 96) * 2;
        let name_hex = hex_string
            .get(start..start + length)
            .ok_or_else(|| ApplyError::InvalidTransaction("Unable to hex for the Name".into()))?;

        let name_bytes = decode(name_hex).map_err(|err| {
            ApplyError::InvalidTransaction(format!("Unable to decode cbor: {}", err))
        })?;

        let name = String::from_utf8(name_bytes).map_err(|err| {
            ApplyError::InvalidTransaction(format!("Unable to decode cbor: {}", err))
        })?;
        start += length;
        let number_type = hex_string.get(start..start + 2).ok_or_else(|| {
            ApplyError::InvalidTransaction("Unable to get hex for Value data".into())
        })?;

        let mut number = usize::from_str_radix(number_type, 16).map_err(|err| {
            ApplyError::InvalidTransaction(format!("Unable to decode cbor: {}", err))
        })?;

        start += 2;
        // For number less than 23 (decimal) the first two bytes represent the number. If it is
        // greater than 23 the first two bytes represent the number of digits required to
        // calculate the value followed by the actual bytes for the number.
        if number > 23 {
            number -= 23;
            let value = match number {
                // two bytes
                1 => {
                    let value = hex_string.get(start..start + 2).ok_or_else(|| {
                        ApplyError::InvalidTransaction("Unable to get number data".into())
                    })?;
                    start += 2;
                    value
                }
                // 4 bytes
                2 => {
                    let value = hex_string.get(start..start + 4).ok_or_else(|| {
                        ApplyError::InvalidTransaction("Unable to get number data".into())
                    })?;
                    start += 4;
                    value
                }
                // 8 bytes
                3 => {
                    let value = hex_string.get(start..start + 8).ok_or_else(|| {
                        ApplyError::InvalidTransaction("Unable to get number data".into())
                    })?;
                    start += 8;
                    value
                }
                // Anymore than 8 bytes is not a u32 and is invalid.
                _ => {
                    return Err(ApplyError::InvalidTransaction(String::from(
                        "Value is too large",
                    )));
                }
            };
            let int_value = u32::from_str_radix(value, 16).map_err(|err| {
                ApplyError::InvalidTransaction(format!("Unable to decode cbor: {}", err))
            })?;
            output.insert(name, int_value);
        } else {
            let int_value = u32::from_str_radix(number_type, 16).map_err(|err| {
                ApplyError::InvalidTransaction(format!("Unable to decode cbor: {}", err))
            })?;
            output.insert(name, int_value);
        }
    }
    Ok(output)
}

fn encode_intkey(map: BTreeMap<String, u32>) -> Result<String, ApplyError> {
    // First two characters should be A followed by the number of elements.
    // Only check for A as this will be a map with 15 or less elements
    // It is unlikely that an address will have that many hash collisions
    let mut hex_string = "A".to_string();
    let map_length = map.len() as u32;
    hex_string = hex_string + &format!("{:X}", map_length);

    let keys: Vec<_> = map.keys().cloned().collect();
    for key in keys {
        // Keys need to have a length between 1 and 20
        let key_length = key.len();
        if !(1..=20).contains(&key_length) {
            return Err(ApplyError::InvalidTransaction(String::from(
                "Key must be at least 1 character and no more than 20",
            )));
        }

        // 96 is equal to 60 hex and is the starting byte for strings.
        let length = 96 + key_length;

        // If value is less then 23, the hex of that number is used as the value.
        // If the value is more then 23 the first two bytes start at hex 18 and increment
        // for more bytes. 18 = 2, 19 = 4, 1A = 8. Should not exeed 8 bytes.
        let encoded_key = encode_upper(key.clone());
        let raw_value = map
            .get(&key)
            .ok_or_else(|| ApplyError::InvalidTransaction("Value from map".into()))?;
        if *raw_value > 23 {
            let mut value = format!("{:02X}", raw_value);
            if value.len() % 2 == 1 {
                value = "0".to_string() + &value.clone();
            }

            let value_length = match value.len() {
                2 => "18",
                4 => "19",
                8 => "1A",
                _ => {
                    return Err(ApplyError::InvalidTransaction(String::from(
                        "Value is too large",
                    )));
                }
            };
            hex_string =
                hex_string + &format!("{:X}", length) + &encoded_key + value_length + &value;
        } else {
            hex_string = hex_string
                + &format!("{:X}", length)
                + &encoded_key
                + &format!("{:02X}", raw_value);
        }
    }
    Ok(hex_string)
}

struct IntkeyPayload {
    name_a: String,
    name_b: String,
    name_c: String,
}

impl IntkeyPayload {
    pub fn new(payload_data: &[u8]) -> Result<Option<IntkeyPayload>, ApplyError> {
        // payload_data should be in the format name_a,name_b,name_c where name_a is the key
        // to start the new value, and name_b and name_c are the existing keys whose values
        // will be multiplied together.
        let payload = String::from_utf8(payload_data.to_vec())
            .map_err(|err| ApplyError::InvalidTransaction(format!("{}", err)))?;
        let payload_vec = payload.split(',').collect::<Vec<&str>>();

        let name_a_raw: String = match payload_vec.first() {
            None => {
                return Err(ApplyError::InvalidTransaction(String::from(
                    "Name A must be a string",
                )));
            }
            Some(name_a_raw) => (*name_a_raw).to_string(),
        };

        if name_a_raw.len() > MAX_NAME_LEN {
            return Err(ApplyError::InvalidTransaction(String::from(
                "Name A must be equal to or less than 20 characters",
            )));
        }

        let name_b_raw: String = match payload_vec.get(1) {
            None => {
                return Err(ApplyError::InvalidTransaction(String::from(
                    "Name B must be a string",
                )));
            }
            Some(name_b_raw) => (*name_b_raw).to_string(),
        };

        if name_b_raw.len() > MAX_NAME_LEN {
            return Err(ApplyError::InvalidTransaction(String::from(
                "Name B must be equal to or less than 20 characters",
            )));
        }

        let name_c_raw: String = match payload_vec.get(2) {
            None => {
                return Err(ApplyError::InvalidTransaction(String::from(
                    "Name C must be a string",
                )));
            }
            Some(name_c_raw) => (*name_c_raw).to_string(),
        };

        if name_c_raw.len() > MAX_NAME_LEN {
            return Err(ApplyError::InvalidTransaction(String::from(
                "Name C must be equal to or less than 20 characters",
            )));
        }

        let intkey_payload = IntkeyPayload {
            name_a: name_a_raw,
            name_b: name_b_raw,
            name_c: name_c_raw,
        };
        Ok(Some(intkey_payload))
    }

    pub fn get_name_a(&self) -> &String {
        &self.name_a
    }

    pub fn get_name_b(&self) -> &String {
        &self.name_b
    }

    pub fn get_name_c(&self) -> &String {
        &self.name_c
    }
}

pub struct IntkeyState<'a> {
    context: &'a mut dyn TransactionContext,
    get_cache: HashMap<String, BTreeMap<String, u32>>,
}

impl<'a> IntkeyState<'a> {
    pub fn new(context: &'a mut dyn TransactionContext) -> IntkeyState {
        IntkeyState {
            context,
            get_cache: HashMap::new(),
        }
    }

    fn calculate_address(name: &str) -> String {
        let mut sha = Sha512::new();
        sha.input(name.as_bytes());
        get_intkey_prefix() + &sha.result_str()[64..]
    }

    pub fn get(&mut self, name: &str) -> Result<Option<u32>, ApplyError> {
        let address = IntkeyState::calculate_address(name);
        let d = self.context.get_state_entry(&address)?;
        match d {
            Some(packed) => {
                let hex_vec: Vec<String> = packed.iter().map(|b| format!("{:02X}", b)).collect();
                let map = decode_intkey(hex_vec.join(""))?;

                let status = match map.get(name) {
                    Some(x) => Ok(Some(*x)),
                    None => Ok(None),
                };
                self.get_cache.insert(address, map);
                status
            }
            None => Ok(None),
        }
    }

    pub fn set(&mut self, name: &str, value: u32) -> Result<(), ApplyError> {
        let mut map: BTreeMap<String, u32> = match self
            .get_cache
            .get_mut(&IntkeyState::calculate_address(name))
        {
            Some(m) => m.clone(),
            None => BTreeMap::new(),
        };
        map.insert(name.into(), value);

        let encoded = encode_intkey(map)?;
        let packed =
            decode(encoded).map_err(|err| ApplyError::InvalidTransaction(format!("{}", err)))?;

        self.context
            .set_state_entry(IntkeyState::calculate_address(name), packed)
            .map_err(|err| ApplyError::InternalError(format!("{}", err)))?;

        Ok(())
    }
}

pub struct IntkeyMultiplyTransactionHandler {
    family_name: String,
    family_versions: Vec<String>,
    namespaces: Vec<String>,
}

impl IntkeyMultiplyTransactionHandler {
    #[allow(clippy::new_without_default)]
    pub fn new() -> IntkeyMultiplyTransactionHandler {
        IntkeyMultiplyTransactionHandler {
            family_name: "intkey_multiply".to_string(),
            family_versions: vec!["1.0".to_string()],
            namespaces: vec![get_intkey_prefix()],
        }
    }
}

impl TransactionHandler for IntkeyMultiplyTransactionHandler {
    fn family_name(&self) -> String {
        self.family_name.clone()
    }

    fn family_versions(&self) -> Vec<String> {
        self.family_versions.clone()
    }

    fn namespaces(&self) -> Vec<String> {
        self.namespaces.clone()
    }

    fn apply(
        &self,
        request: &TpProcessRequest,
        context: &mut dyn TransactionContext,
    ) -> Result<(), ApplyError> {
        let payload = IntkeyPayload::new(request.get_payload());
        let payload = match payload {
            Err(e) => return Err(e),
            Ok(payload) => payload,
        };
        let payload = match payload {
            Some(x) => x,
            None => {
                return Err(ApplyError::InvalidTransaction(String::from(
                    "Request must contain a payload",
                )));
            }
        };
        let mut state = IntkeyState::new(context);
        info!(
            "payload: {} {} {}",
            payload.get_name_a(),
            payload.get_name_b(),
            payload.get_name_c()
        );

        match state.get(payload.get_name_a()) {
            Ok(None) => (),
            Ok(Some(_)) => {
                return Err(ApplyError::InvalidTransaction(format!(
                    "{} is already set",
                    payload.get_name_a()
                )));
            }
            Err(err) => return Err(err),
        };

        let orig_value_b: u64 = match state.get(payload.get_name_b()) {
            Ok(Some(v)) => u64::from(v),
            Ok(None) => {
                return Err(ApplyError::InvalidTransaction(String::from(
                    "Multiply requires a set value for name_b",
                )));
            }
            Err(err) => return Err(err),
        };

        let orig_value_c: u64 = match state.get(payload.get_name_c()) {
            Ok(Some(v)) => u64::from(v),
            Ok(None) => {
                return Err(ApplyError::InvalidTransaction(String::from(
                    "Multiply requires a set value for name_c",
                )));
            }
            Err(err) => return Err(err),
        };
        let new_value = orig_value_b * orig_value_c;
        if new_value > u64::from(MAX_VALUE) {
            return Err(ApplyError::InvalidTransaction(format!(
                "Multiplied value is larger then max allowed: {}",
                new_value
            )));
        };
        state.set(payload.get_name_a(), new_value as u32)?;

        // Send an event with the result and the current values of b & c
        // Compute the event data to be sent in Bvalue,Cvalue,Avalue format
        let values = format!("{},{},{}", orig_value_b, orig_value_c, new_value);
        context
            .add_event(String::from("computation"), Vec::new(), values.as_bytes())
            .map_err(ApplyError::from)
    }
}

#[cfg(target_arch = "wasm32")]
// Sabre apply must return a bool
fn apply(
    request: &TpProcessRequest,
    context: &mut dyn TransactionContext,
) -> Result<bool, ApplyError> {
    let handler = IntkeyMultiplyTransactionHandler::new();
    match handler.apply(request, context) {
        Ok(_) => Ok(true),
        Err(err) => {
            info!("{}", err);
            Err(err)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub unsafe fn entrypoint(payload: WasmPtr, signer: WasmPtr, signature: WasmPtr) -> i32 {
    execute_entrypoint(payload, signer, signature, apply)
}
