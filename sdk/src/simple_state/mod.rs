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

pub mod addresser;
pub mod context;
pub mod error;

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::protocol::simple_state::ValueType;
    use crate::{TransactionContext, WasmSdkError};

    use addresser::{DoubleKeyHashAddresser, KeyHashAddresser};
    use context::KeyValueTransactionContext;
    use error::SimpleStateError;

    struct TestState {
        state: HashMap<String, Vec<u8>>,
    }

    impl TestState {
        pub fn new() -> Self {
            TestState {
                state: HashMap::new(),
            }
        }

        fn get_entries(
            &self,
            addresses: &[String],
        ) -> Result<Vec<(String, Vec<u8>)>, SimpleStateError> {
            let mut values = Vec::new();
            addresses.iter().for_each(|key| {
                if let Some(value) = self.state.get(key) {
                    values.push((key.to_string(), value.to_vec()))
                }
            });
            Ok(values)
        }

        fn set_entries(&mut self, entries: Vec<(String, Vec<u8>)>) -> Result<(), SimpleStateError> {
            entries.iter().for_each(|(key, value)| {
                match self.state.insert(key.to_string(), value.to_vec()) {
                    _ => (),
                }
            });
            Ok(())
        }

        fn delete_entries(
            &mut self,
            addresses: &[String],
        ) -> Result<Vec<String>, SimpleStateError> {
            let mut deleted = Vec::new();
            addresses.iter().for_each(|key| {
                if let Some(_) = self.state.remove(key) {
                    deleted.push(key.to_string());
                }
            });
            Ok(deleted)
        }
    }

    struct TestContext {
        internal_state: Arc<Mutex<TestState>>,
    }

    impl TestContext {
        pub fn new() -> Self {
            TestContext {
                internal_state: Arc::new(Mutex::new(TestState::new())),
            }
        }
    }

    impl TransactionContext for TestContext {
        fn get_state_entries(
            &self,
            addresses: &[String],
        ) -> Result<Vec<(String, Vec<u8>)>, WasmSdkError> {
            self.internal_state
                .lock()
                .expect("Test lock was poisoned in get method")
                .get_entries(addresses)
                .map_err(|err| {
                    WasmSdkError::InternalError(format!(
                        "Unable to get state entries: {}",
                        err.to_string(),
                    ))
                })
        }

        fn set_state_entries(&self, entries: Vec<(String, Vec<u8>)>) -> Result<(), WasmSdkError> {
            self.internal_state
                .lock()
                .expect("Test lock was poisoned in set method")
                .set_entries(entries)
                .map_err(|err| {
                    WasmSdkError::InternalError(format!(
                        "Unable to get state entries: {}",
                        err.to_string(),
                    ))
                })
        }

        fn delete_state_entries(&self, addresses: &[String]) -> Result<Vec<String>, WasmSdkError> {
            self.internal_state
                .lock()
                .expect("Test lock was poisoned in delete method")
                .delete_entries(addresses)
                .map_err(|err| {
                    WasmSdkError::InternalError(format!(
                        "Unable to get state entries: {}",
                        err.to_string(),
                    ))
                })
        }
    }

    fn create_entry_value_map(key: String, value: ValueType) -> HashMap<String, ValueType> {
        let mut value_map = HashMap::new();
        value_map.insert(key, value);
        value_map
    }

    #[test]
    // Check that the KeyValueTransactionContext set_state_entry method successfully sets
    // the state entry. Uses the KeyHashAddresser implementation.
    fn test_simple_set_state_entry() {
        let mut context = TestContext::new();
        let addresser = KeyHashAddresser::new("prefix".to_string());
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let value = ValueType::Int32(32);
        let mut state_value = HashMap::new();
        state_value.insert("key1".to_string(), value);

        assert!(simple_state
            .set_state_entry(&"a".to_string(), state_value)
            .is_ok());
    }

    #[test]
    // Check that the KeyValueTransactionContext get_state_entry method successfully fetches
    // the correct state entry. Uses the KeyHashAddresser implementation.
    fn test_simple_get_state_entry() {
        let mut context = TestContext::new();
        let addresser = KeyHashAddresser::new("prefix".to_string());
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let value = ValueType::Int32(32);
        let mut state_value = HashMap::new();
        state_value.insert("key1".to_string(), value);
        simple_state
            .set_state_entry(&"a".to_string(), state_value)
            .expect("Unable to set state entry in get_state_entry test");

        let values = simple_state.get_state_entry(&"a".to_string()).unwrap();
        assert!(values.is_some());
        assert_eq!(
            values.unwrap().get(&"key1".to_string()),
            Some(&ValueType::Int32(32))
        );
    }

    #[test]
    // Check that the KeyValueTransactionContext delete_state_entry method successfully deletes
    // the state entry. Uses the KeyHashAddresser implementation.
    fn test_simple_delete_state_entry() {
        let mut context = TestContext::new();
        let addresser = KeyHashAddresser::new("prefix".to_string());
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let value = ValueType::Int32(32);
        let mut state_value = HashMap::new();
        state_value.insert("key1".to_string(), value);

        simple_state
            .set_state_entry(&"a".to_string(), state_value)
            .expect("Unable to set state entry in delete_state_entry test");

        simple_state
            .get_state_entry(&"a".to_string())
            .expect("Unable to get state entry in delete_state_entries test");

        let deleted = simple_state.delete_state_entry("a".to_string()).unwrap();
        assert!(deleted.is_some());
        assert_eq!(deleted, Some("a".to_string()));

        let already_deleted = simple_state.delete_state_entry("a".to_string()).unwrap();
        assert!(already_deleted.is_none());
        let deleted_value = simple_state.get_state_entry(&"a".to_string()).unwrap();
        assert!(deleted_value.is_none());
    }

    #[test]
    // Check that the KeyValueTransactionContext set_state_entries method successfully sets
    // the state entries. Uses the KeyHashAddresser implementation.
    fn test_simple_set_state_entries() {
        let mut context = TestContext::new();
        let addresser = KeyHashAddresser::new("prefix".to_string());
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let mut entries = HashMap::new();

        let first_key = &"a".to_string();
        let first_value_map = create_entry_value_map("key1".to_string(), ValueType::Int32(32));
        let second_key = &"b".to_string();
        let second_value_map =
            create_entry_value_map("key1".to_string(), ValueType::String("String".to_string()));

        entries.insert(first_key, first_value_map);
        entries.insert(second_key, second_value_map);

        let set_result = simple_state.set_state_entries(entries);
        assert!(set_result.is_ok());
    }

    #[test]
    // Check that the KeyValueTransactionContext get_state_entries method successfully gets
    // the state entries. Uses the KeyHashAddresser implementation.
    fn test_simple_get_state_entries() {
        let mut context = TestContext::new();
        let addresser = KeyHashAddresser::new("prefix".to_string());
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let mut entries = HashMap::new();

        let first_key = &"a".to_string();
        let first_value_map = create_entry_value_map("key1".to_string(), ValueType::Int32(32));
        let second_key = &"b".to_string();
        let second_value_map =
            create_entry_value_map("key1".to_string(), ValueType::String("String".to_string()));

        entries.insert(first_key, first_value_map.clone());
        entries.insert(second_key, second_value_map.clone());

        simple_state
            .set_state_entries(entries)
            .expect("Unable to set state entries in get_state_entries test");

        let values = simple_state
            .get_state_entries([first_key, second_key].to_vec())
            .unwrap();
        assert_eq!(values.get("a").unwrap(), &first_value_map);
        assert_eq!(values.get("b").unwrap(), &second_value_map);
    }

    #[test]
    // Check that the KeyValueTransactionContext delete_state_entries method successfully deletes
    // the state entries. Uses the KeyHashAddresser implementation.
    fn test_simple_delete_state_entries() {
        let mut context = TestContext::new();
        let addresser = KeyHashAddresser::new("prefix".to_string());
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let mut entries = HashMap::new();

        let first_key = &"a".to_string();
        let first_value_map = create_entry_value_map("key1".to_string(), ValueType::Int32(32));
        let second_key = &"b".to_string();
        let second_value_map =
            create_entry_value_map("key1".to_string(), ValueType::String("String".to_string()));

        entries.insert(first_key, first_value_map.clone());
        entries.insert(second_key, second_value_map.clone());

        simple_state
            .set_state_entries(entries)
            .expect("Unable to set state entries in get_state_entries test");

        simple_state
            .get_state_entries([first_key, second_key].to_vec())
            .expect("Unable to get state entries in delete_state_entries test");

        let deleted = simple_state
            .delete_state_entries(["a".to_string(), "b".to_string()].to_vec())
            .expect("Unable to delete state entries");
        assert!(deleted.contains(&"a".to_string()));
        assert!(deleted.contains(&"b".to_string()));

        let already_deleted = simple_state
            .delete_state_entries(["a".to_string(), "b".to_string()].to_vec())
            .unwrap();
        assert!(already_deleted.is_empty());
    }

    #[test]
    // Check that the KeyValueTransactionContext set_state_entry method successfully sets
    // the correct state entry. Uses the DoubleKeyHashAddresser implementation.
    fn test_double_set_state_entry() {
        let mut context = TestContext::new();
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), None);
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let value = ValueType::Int64(64);
        let mut state_value = HashMap::new();
        state_value.insert("key1".to_string(), value);

        assert!(simple_state
            .set_state_entry(&("a".to_string(), "b".to_string()), state_value)
            .is_ok());
    }

    #[test]
    // Check that the KeyValueTransactionContext get_state_entry method successfully gets
    // the correct state entry. Uses the DoubleKeyHashAddresser implementation.
    fn test_double_get_state_entry() {
        let mut context = TestContext::new();
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), None);
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let value = ValueType::Int64(64);
        let mut state_value = HashMap::new();
        state_value.insert("key1".to_string(), value);
        simple_state
            .set_state_entry(&("a".to_string(), "b".to_string()), state_value)
            .expect("Unable to set state entry in get_state_entry test");

        let values = simple_state
            .get_state_entry(&("a".to_string(), "b".to_string()))
            .unwrap();
        assert!(values.is_some());
        assert_eq!(
            values.unwrap().get(&"key1".to_string()),
            Some(&ValueType::Int64(64))
        );
    }

    #[test]
    // Check that the KeyValueTransactionContext delete_state_entry method successfully deletes
    // the state entry. Uses the DoubleKeyHashAddresser implementation.
    fn test_double_delete_state_entry() {
        let mut context = TestContext::new();
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), None);
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let value = ValueType::Int64(64);
        let mut state_value = HashMap::new();
        state_value.insert("key1".to_string(), value);
        simple_state
            .set_state_entry(&("a".to_string(), "b".to_string()), state_value)
            .expect("Unable to set state entry in get_state_entry test");
        simple_state
            .get_state_entry(&("a".to_string(), "b".to_string()))
            .expect("Unable to get state entry in delete_state_entries test");

        let deleted = simple_state
            .delete_state_entry(("a".to_string(), "b".to_string()))
            .unwrap();
        assert!(deleted.is_some());
        assert_eq!(deleted, Some(format!("{}_{}", "a", "b")));
    }

    #[test]
    // Check that the KeyValueTransactionContext set_state_entries method successfully sets
    // the state entries. Uses the DoubleKeyHashAddresser implementation.
    fn test_double_set_state_entries() {
        let mut context = TestContext::new();
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), None);
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let mut entries = HashMap::new();
        let first_key = &("a".to_string(), "b".to_string());
        let first_value_map = create_entry_value_map("key1".to_string(), ValueType::Int32(32));
        let second_key = &("c".to_string(), "d".to_string());
        let second_value_map =
            create_entry_value_map("key1".to_string(), ValueType::String("String".to_string()));
        entries.insert(first_key, first_value_map);
        entries.insert(second_key, second_value_map);

        assert!(simple_state.set_state_entries(entries).is_ok());
    }

    #[test]
    // Check that the KeyValueTransactionContext get_state_entries method successfully gets
    // the state entries. Uses the DoubleKeyHashAddresser implementation.
    fn test_double_get_state_entries() {
        let mut context = TestContext::new();
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), None);
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let mut entries = HashMap::new();
        let first_key = &("a".to_string(), "b".to_string());
        let first_value_map = create_entry_value_map("key1".to_string(), ValueType::Int32(32));
        let second_key = &("c".to_string(), "d".to_string());
        let second_value_map =
            create_entry_value_map("key1".to_string(), ValueType::String("String".to_string()));
        entries.insert(first_key, first_value_map.clone());
        entries.insert(second_key, second_value_map.clone());
        simple_state
            .set_state_entries(entries)
            .expect("Unable to set_state_entries in get_state_entries test");

        let values = simple_state
            .get_state_entries([first_key, second_key].to_vec())
            .expect("Unable to get state entries in get_state_entries test");
        assert_eq!(
            values.get(&format!("{}_{}", "a", "b")).unwrap(),
            &first_value_map
        );
        assert_eq!(
            values.get(&format!("{}_{}", "c", "d")).unwrap(),
            &second_value_map
        );
    }

    #[test]
    // Check that the KeyValueTransactionContext delete_state_entries method successfully deletes
    // the state entries. Uses the DoubleKeyHashAddresser implementation.
    fn test_double_delete_state_entries() {
        let mut context = TestContext::new();
        let addresser = DoubleKeyHashAddresser::new("prefix".to_string(), None);
        let simple_state = KeyValueTransactionContext::new(&mut context, addresser);

        let mut entries = HashMap::new();
        let first_key = ("a".to_string(), "b".to_string());
        let first_value_map = create_entry_value_map("key1".to_string(), ValueType::Int32(32));
        let second_key = ("c".to_string(), "d".to_string());
        let second_value_map =
            create_entry_value_map("key1".to_string(), ValueType::String("String".to_string()));
        entries.insert(&first_key, first_value_map.clone());
        entries.insert(&second_key, second_value_map.clone());
        simple_state
            .set_state_entries(entries)
            .expect("Unable to set_state_entries in delete_state_entries test");
        simple_state
            .get_state_entries([&first_key, &second_key].to_vec())
            .expect("Unable to get_state_entries in the delete_state_entries test");

        let deleted = simple_state
            .delete_state_entries([first_key, second_key].to_vec())
            .expect("Unable to delete state entries");
        assert!(deleted.contains(&format!("{}_{}", "a", "b")));
        assert!(deleted.contains(&format!("{}_{}", "c", "d")));
    }
}
