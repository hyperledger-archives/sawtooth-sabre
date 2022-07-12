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

//! Contains functions which assist with batch submission to a REST API

use reqwest::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Url,
};
use std::fmt;

use transact::{protocol::batch::Batch, protos::IntoBytes};

use crate::error::CliError;

pub fn submit_batches(url: &str, batch_list: Vec<Batch>) -> Result<String, CliError> {
    let url = Url::parse(&format!("{}/batches", url))
        .map_err(|e| CliError::User(format!("Invalid URL: {}: {}", e, url)))?;

    match url.scheme() {
        "http" => (),
        "" => return Err(CliError::User(format!("No scheme in URL: {}", url))),
        s => {
            return Err(CliError::User(format!(
                "Unsupported scheme ({}) in URL: {}",
                s, url
            )))
        }
    }

    let bytes = batch_list.into_bytes()?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url)
        .header(CONTENT_TYPE, "application/octet-stream")
        .header(CONTENT_LENGTH, bytes.len())
        .body(bytes)
        .send()?
        .json::<Link>()?;

    println!("Response Body:\n{:?}", response);

    Ok(response.link)
}

pub fn wait_for_batch(url: &str, wait: u64) -> Result<StatusResponse, CliError> {
    let url = Url::parse(&format!("{url}&wait={wait}", url = url, wait = wait))
        .map_err(|e| CliError::User(format!("Invalid URL: {}: {}", e, url)))?;

    match url.scheme() {
        "http" => (),
        "" => return Err(CliError::User(format!("No scheme in URL: {}", url))),
        s => {
            return Err(CliError::User(format!(
                "Unsupported scheme ({}) in URL: {}",
                s, url
            )))
        }
    }

    let response = reqwest::blocking::get(url)?.json::<StatusResponse>()?;

    Ok(response)
}

#[derive(Deserialize, Debug, PartialEq)]
struct Link {
    link: String,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct BatchStatus {
    id: String,
    status: String,
    invalid_transactions: Vec<InvalidTransaction>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct InvalidTransaction {
    id: String,
    message: String,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct StatusResponse {
    data: Vec<BatchStatus>,
    link: String,
}

impl StatusResponse {
    pub fn is_finished(&self) -> bool {
        self.data.iter().all(|x| x.status == "COMMITTED")
            || self.data.iter().any(|x| x.status == "INVALID")
    }
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\"link\": {}}}", self.link)
    }
}

impl fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut invalid_txn_string_vec = Vec::new();
        for txn in &self.invalid_transactions {
            invalid_txn_string_vec.push(txn.to_string());
        }
        write!(
            f,
            "{{\"id\": \"{}\", \"status\": \"{}\", \"invalid_transactions\": [{}]}}",
            self.id,
            self.status,
            invalid_txn_string_vec.join(",")
        )
    }
}

impl fmt::Display for InvalidTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{\"id\": \"{}\", \"message\": \"{}\"}}",
            self.id, self.message
        )
    }
}

impl fmt::Display for StatusResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut data_string_vec = Vec::new();
        for data in &self.data {
            data_string_vec.push(data.to_string());
        }

        write!(
            f,
            "StatusResponse {{\"data\":[{}], \"link\": \"{}\"}}",
            data_string_vec.join(","),
            self.link
        )
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use cylinder::{secp256k1::Secp256k1Context, Context};
    use mockito;
    use transact::protocol::{
        batch::{Batch, BatchBuilder},
        transaction::{HashMethod, TransactionBuilder},
    };

    fn parse_hex(hex: &str) -> Vec<u8> {
        let mut res = vec![];
        for i in (0..hex.len()).step_by(2) {
            res.push(u8::from_str_radix(&hex[i..i + 2], 16).unwrap());
        }
        res
    }

    fn mock_key(x: u8) -> String {
        x.to_string().repeat(66)
    }

    struct MockBatch {}

    impl MockBatch {
        fn new() -> Batch {
            let context = Secp256k1Context::new();
            let key = context.new_random_private_key();
            let signer = context.new_signer(key);

            let transaction = TransactionBuilder::new()
                .with_batcher_public_key(parse_hex(&mock_key(1)))
                .with_dependencies(vec![mock_key(2), mock_key(3)])
                .with_family_name("test_family".to_string())
                .with_family_version("0.1".to_string())
                .with_inputs(vec![parse_hex(&mock_key(4)), parse_hex(&mock_key(5)[0..4])])
                .with_nonce("f9kdzz".to_string().into_bytes())
                .with_outputs(vec![parse_hex(&mock_key(6)), parse_hex(&mock_key(7)[0..4])])
                .with_payload_hash_method(HashMethod::Sha512)
                .with_payload(vec![0x05, 0x06, 0x07, 0x08])
                .build(&*signer)
                .expect("Failed to build transaction");

            BatchBuilder::new()
                .with_transactions(vec![transaction])
                .build(&*signer)
                .expect("Failed to build transact batch")
        }
    }

    #[test]
    // Asserts that URLs with a scheme other that http return an error
    fn test_cli_submit_batches_scheme() {
        assert!(submit_batches("https://test.com", vec![MockBatch::new()]).is_err());
        assert!(submit_batches("file://test", vec![MockBatch::new()]).is_err());
    }

    #[test]
    // Asserts that submit_batches() returns data as expected
    fn test_cli_submit_batches() {
        let url = mockito::server_url();
        let _m1 = mockito::mock("POST", "/batches")
            .with_body("{\"link\":\"test.com/success\"}")
            .create();
        let expected = "test.com/success".to_string();
        let result = submit_batches(&url, vec![MockBatch::new()]);

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    // Asserts that URLs with a scheme other that http return an error
    fn test_cli_wait_for_batches_scheme() {
        assert!(submit_batches("https://test.com", vec![MockBatch::new()]).is_err());
        assert!(submit_batches("file://test", vec![MockBatch::new()]).is_err());
    }

    #[test]
    // Asserts that wait_for_batch() returns data as expected
    fn test_cli_wait_for_batch() {
        let url = mockito::server_url();
        let _m1 = mockito::mock("GET", "/test?foo=bar&wait=30")
            .with_body("{\"data\":[], \"link\":\"test.com/success\"}")
            .create();
        let expected = StatusResponse {
            data: Vec::new(),
            link: "test.com/success".to_string(),
        };
        let result = wait_for_batch(&format!("{}/test?foo=bar", &url), 30);

        assert_eq!(result.unwrap(), expected);
    }
}
