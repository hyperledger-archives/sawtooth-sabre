// Copyright 2019 Cargill Incorporated
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

use crate::simple_state::error::SimpleStateError;

use crypto::digest::Digest;
use crypto::sha2::Sha512;

pub const ADDRESS_LENGTH: usize = 70;

pub trait Addresser<K> {
    /// Returns a radix address calculated from the given keys
    ///
    /// # Arguments
    ///
    /// * `keys` - Contains natural keys used to calculate an address
    ///
    fn compute(&self, keys: &K) -> Result<String, SimpleStateError>;

    /// Returns a human readable string of the given keys
    ///
    /// # Arguments
    ///
    /// * `keys` - Contains natural keys
    ///
    fn normalize(&self, keys: &K) -> String;
}

fn hash(hash_length: usize, key: &str) -> String {
    let mut sha = Sha512::new();
    sha.input(key.as_bytes());
    sha.result_str()[..hash_length].to_string()
}

pub struct KeyHashAddresser {
    prefix: String,
}

impl KeyHashAddresser {
    pub fn new(prefix: String) -> KeyHashAddresser {
        KeyHashAddresser { prefix }
    }
}

impl Addresser<String> for KeyHashAddresser {
    fn compute(&self, keys: &String) -> Result<String, SimpleStateError> {
        let hash_length = ADDRESS_LENGTH - self.prefix.len();

        Ok(String::from(&self.prefix) + &hash(hash_length, keys))
    }

    fn normalize(&self, key: &String) -> String {
        key.to_string()
    }
}

pub struct DoubleKeyHashAddresser {
    prefix: String,
    first_hash_length: usize,
}

impl DoubleKeyHashAddresser {
    pub fn new(prefix: String, first_hash_length: Option<usize>) -> DoubleKeyHashAddresser {
        DoubleKeyHashAddresser {
            prefix: prefix.clone(),
            first_hash_length: first_hash_length.unwrap_or((ADDRESS_LENGTH - prefix.len()) / 2),
        }
    }
}

impl Addresser<(String, String)> for DoubleKeyHashAddresser {
    fn compute(&self, keys: &(String, String)) -> Result<String, SimpleStateError> {
        let hash_length = ADDRESS_LENGTH - self.prefix.len();
        let second_hash_length = hash_length - self.first_hash_length;
        if (self.prefix.len() + self.first_hash_length + second_hash_length) != ADDRESS_LENGTH {
            return Err(SimpleStateError::AddresserError(
                "Incorrect hash length".to_string(),
            ));
        }
        let first_hash = &hash(self.first_hash_length, &keys.0);
        let second_hash = &hash(second_hash_length, &keys.1);

        Ok(String::from(&self.prefix) + first_hash + second_hash)
    }

    fn normalize(&self, keys: &(String, String)) -> String {
        keys.0.to_string() + "_" + &keys.1
    }
}
