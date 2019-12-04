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

pub struct TripleKeyHashAddresser {
    prefix: String,
    first_hash_length: usize,
    second_hash_length: usize,
}

impl TripleKeyHashAddresser {
    pub fn new(
        prefix: String,
        first_hash_length: Option<usize>,
        second_hash_length: Option<usize>,
    ) -> TripleKeyHashAddresser {
        let (first, second) =
            calculate_hash_lengths(prefix.len(), first_hash_length, second_hash_length);
        TripleKeyHashAddresser {
            prefix,
            first_hash_length: first,
            second_hash_length: second,
        }
    }
}

impl Addresser<(String, String, String)> for TripleKeyHashAddresser {
    fn compute(&self, keys: &(String, String, String)) -> Result<String, SimpleStateError> {
        let hash_length = ADDRESS_LENGTH - self.prefix.len();
        let last_hash_length = hash_length - (self.first_hash_length + self.second_hash_length);
        if (self.prefix.len() + self.first_hash_length + self.second_hash_length + last_hash_length)
            != ADDRESS_LENGTH
        {
            return Err(SimpleStateError::AddresserError(
                "Incorrect hash length".to_string(),
            ));
        }

        let first_hash = &hash(self.first_hash_length, &keys.0);
        let second_hash = &hash(self.second_hash_length, &keys.1);
        let third_hash = &hash(last_hash_length, &keys.2);

        Ok(String::from(&self.prefix) + first_hash + second_hash + third_hash)
    }

    fn normalize(&self, keys: &(String, String, String)) -> String {
        keys.0.to_string() + "_" + &keys.1 + "_" + &keys.2
    }
}

// Used to calculate the lengths of the key hashes to be used to create an address by the
// TripleKeyHashAddresser.
fn calculate_hash_lengths(
    prefix_length: usize,
    first_length: Option<usize>,
    second_length: Option<usize>,
) -> (usize, usize) {
    match (first_length, second_length) {
        (Some(first), Some(second)) => (first, second),
        (None, Some(second)) => (((ADDRESS_LENGTH - prefix_length - second) / 2), second),
        (Some(first), None) => (first, ((ADDRESS_LENGTH - prefix_length - first) / 2)),
        (None, None) => (
            ((ADDRESS_LENGTH - prefix_length) / 3),
            ((ADDRESS_LENGTH - prefix_length) / 3),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // check that the KeyHashAddresser creates a valid radix address with the correct prefix
    // and valid length
    fn test_key_hash_addresser() {
        let addresser = KeyHashAddresser::new("prefix".to_string());
        let addr = addresser.compute(&"a".to_string()).unwrap();
        assert_eq!(addr[..6], "prefix".to_string());
        assert_eq!(addr.len(), 70);

        let key_hash = hash(64, "a");
        assert_eq!(addr[6..70], key_hash[..64]);

        let normalized = addresser.normalize(&"b".to_string());
        assert_eq!(normalized, "b".to_string());
    }

    #[test]
    // check that the DoubleKeyHashAddresser creates a valid radix address with the correct prefix
    // and valid default length, with a key represented as a tuple with two natural keys
    fn test_double_key_default_length() {
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), None);
        let key1 = "a";
        let key1_hash = hash(32, key1);
        let key2 = "b";
        let key2_hash = hash(32, key2);

        let addr = addresser
            .compute(&(key1.to_string(), key2.to_string()))
            .unwrap();
        assert_eq!(addr[..6], "prefix".to_string());
        assert_eq!(addr.len(), 70);

        assert_eq!(addr[6..38], key1_hash[..32]);
        assert_eq!(addr[38..], key2_hash[..32]);

        let normalized = addresser.normalize(&(key1.to_string(), key2.to_string()));
        assert_eq!(normalized, "a_b".to_string());
    }

    #[test]
    // check that the DoubleKeyHashAddresser creates a valid radix address with the correct prefix
    // and valid first hash length of 16, with a key represented as a tuple with two natural keys
    fn test_double_key_custom_length() {
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), Some(16));
        let key1 = "a";
        let key1_hash = hash(16, key1);
        let key2 = "b";
        let key2_hash = hash(48, key2);

        let addr = addresser
            .compute(&(key1.to_string(), key2.to_string()))
            .unwrap();
        assert_eq!(addr[..6], "prefix".to_string());
        assert_eq!(addr.len(), 70);

        assert_eq!(addr[6..22], key1_hash[..16]);
        assert_eq!(addr[22..], key2_hash[..48]);

        let normalized = addresser.normalize(&(key1.to_string(), key2.to_string()));
        assert_eq!(normalized, "a_b".to_string());
    }

    #[test]
    // check that the TripleKeyHashAddresser creates a valid radix address with the correct prefix
    // and valid default lengths, with a key represented as a tuple with three natural keys
    fn test_triple_key_default_length() {
        let addresser = TripleKeyHashAddresser::new("prefix".to_string(), None, None);
        let key1 = "a";
        let key1_hash = hash(21, key1);
        let key2 = "b";
        let key2_hash = hash(21, key2);
        let key3 = "c";
        let key3_hash = hash(22, key3);

        let addr = addresser
            .compute(&(key1.to_string(), key2.to_string(), key3.to_string()))
            .unwrap();
        assert_eq!(addr[..6], "prefix".to_string());
        assert_eq!(addr.len(), 70);

        assert_eq!(addr[6..27], key1_hash[..21]);
        assert_eq!(addr[27..48], key2_hash[..21]);
        assert_eq!(addr[48..], key3_hash[..22]);

        let normalized =
            addresser.normalize(&(key1.to_string(), key2.to_string(), key3.to_string()));
        assert_eq!(normalized, "a_b_c".to_string());
    }

    #[test]
    // check that the TripleKeyHashAddresser creates a valid radix address with the correct prefix
    // and valid first hash length of 14 and second and third hash length of 25,
    // with a key represented as a tuple with three natural keys
    fn test_triple_key_custom_first_length() {
        let addresser = TripleKeyHashAddresser::new("prefix".to_string(), Some(14), None);
        let key1 = "a";
        let key1_hash = hash(14, key1);
        let key2 = "b";
        let key2_hash = hash(25, key2);
        let key3 = "c";
        let key3_hash = hash(25, key3);

        let addr = addresser
            .compute(&(key1.to_string(), key2.to_string(), key3.to_string()))
            .unwrap();
        assert_eq!(addr[..6], "prefix".to_string());
        assert_eq!(addr.len(), 70);

        assert_eq!(addr[6..20], key1_hash[..14]);
        assert_eq!(addr[20..45], key2_hash[..25]);
        assert_eq!(addr[45..], key3_hash[..25]);

        let normalized =
            addresser.normalize(&(key1.to_string(), key2.to_string(), key3.to_string()));
        assert_eq!(normalized, "a_b_c".to_string());
    }

    #[test]
    // check that the TripleKeyHashAddresser creates a valid radix address with the correct prefix
    // and valid first and last hash length of 25 and second hash length of 14,
    // with a key represented as a tuple with three natural keys
    fn test_triple_key_custom_second_length() {
        let addresser = TripleKeyHashAddresser::new("prefix".to_string(), None, Some(14));
        let key1 = "a";
        let key1_hash = hash(25, key1);
        let key2 = "b";
        let key2_hash = hash(14, key2);
        let key3 = "c";
        let key3_hash = hash(25, key3);

        let addr = addresser
            .compute(&(key1.to_string(), key2.to_string(), key3.to_string()))
            .unwrap();
        assert_eq!(addr[..6], "prefix".to_string());
        assert_eq!(addr.len(), 70);

        assert_eq!(addr[6..31], key1_hash[..25]);
        assert_eq!(addr[31..45], key2_hash[..14]);
        assert_eq!(addr[45..], key3_hash[..25]);

        let normalized =
            addresser.normalize(&(key1.to_string(), key2.to_string(), key3.to_string()));
        assert_eq!(normalized, "a_b_c".to_string());
    }

    #[test]
    // check that the TripleKeyHashAddresser creates a valid radix address with the correct prefix
    // and valid first and second length of 10 and last hash length of 44,
    // with a key represented as a tuple with three natural keys
    fn test_triple_key_custom_lengths() {
        let addresser = TripleKeyHashAddresser::new("prefix".to_string(), Some(10), Some(10));
        let key1 = "a";
        let key1_hash = hash(10, key1);
        let key2 = "b";
        let key2_hash = hash(10, key2);
        let key3 = "c";
        let key3_hash = hash(44, key3);

        let addr = addresser
            .compute(&(key1.to_string(), key2.to_string(), key3.to_string()))
            .unwrap();
        assert_eq!(addr[..6], "prefix".to_string());
        assert_eq!(addr.len(), 70);

        assert_eq!(addr[6..16], key1_hash[..10]);
        assert_eq!(addr[16..26], key2_hash[..10]);
        assert_eq!(addr[26..], key3_hash[..44]);

        let normalized =
            addresser.normalize(&(key1.to_string(), key2.to_string(), key3.to_string()));
        assert_eq!(normalized, "a_b_c".to_string());
    }

    #[test]
    // Tests the calculate_hash_lengths function using several different custom first hash lengths
    // and `None` for the second length.
    fn test_calculate_hash_custom_first_length() {
        let (first_length, second_length) = calculate_hash_lengths(6, Some(21), None);
        assert_eq!(first_length, 21);
        assert_eq!(second_length, (43 / 2));

        let (first_length, second_length) = calculate_hash_lengths(6, Some(41), None);
        assert_eq!(first_length, 41);
        assert_eq!(second_length, (23 / 2));

        let (first_length, second_length) = calculate_hash_lengths(6, Some(61), None);
        assert_eq!(first_length, 61);
        assert_eq!(second_length, (3 / 2));
    }

    #[test]
    // Tests the calculate_hash_lengths function using `None` for the first hash length and several
    // custom values for the second length.
    fn test_calculate_hash_custom_second_length() {
        let (first_length, second_length) = calculate_hash_lengths(6, None, Some(21));
        assert_eq!(first_length, (43 / 2));
        assert_eq!(second_length, 21);

        let (first_length, second_length) = calculate_hash_lengths(6, None, Some(41));
        assert_eq!(first_length, (23 / 2));
        assert_eq!(second_length, 41);

        let (first_length, second_length) = calculate_hash_lengths(6, None, Some(61));
        assert_eq!(first_length, (3 / 2));
        assert_eq!(second_length, 61);
    }

    #[test]
    // Tests the calculate_hash_lengths function using several different custom hash lengths.
    fn test_calculate_hash_custom_lengths() {
        let (first_length, second_length) = calculate_hash_lengths(6, Some(42), Some(12));
        assert_eq!(first_length, 42);
        assert_eq!(second_length, 12);

        let (first_length, second_length) = calculate_hash_lengths(6, Some(12), Some(42));
        assert_eq!(first_length, 12);
        assert_eq!(second_length, 42);

        let (first_length, second_length) = calculate_hash_lengths(6, Some(20), Some(20));
        assert_eq!(first_length, 20);
        assert_eq!(second_length, 20);
    }

    #[test]
    // Tests the calculate_hash_lengths function using `None` for both hash lengths.
    fn test_calculate_hash_no_custom_lengths() {
        let (first_length, second_length) = calculate_hash_lengths(6, None, None);
        assert_eq!(first_length, 21);
        assert_eq!(second_length, 21);

        let (first_length, second_length) = calculate_hash_lengths(30, None, None);
        assert_eq!(first_length, (40 / 3));
        assert_eq!(second_length, (40 / 3));

        let (first_length, second_length) = calculate_hash_lengths(50, None, None);
        assert_eq!(first_length, (20 / 3));
        assert_eq!(second_length, (20 / 3));
    }
}
