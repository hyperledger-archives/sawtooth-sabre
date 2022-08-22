// Copyright 2018-2021 Cargill Incorporated
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

//! Contains functions which assist with fetching state

use reqwest::Url;

use crate::error::CliError;

pub fn get_state_with_prefix(url: &str, prefix: &str) -> Result<Vec<StateEntry>, CliError> {
    let url = Url::parse(&format!(
        "{url}/state?address={prefix}",
        url = url,
        prefix = prefix
    ))
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

    let response = reqwest::blocking::get(url)?.json::<JsonStateEntry>()?;

    Ok(response.data)
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonStateEntry {
    data: Vec<StateEntry>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StateEntry {
    pub address: String,
    pub data: String,
}

#[cfg(test)]
mod tests {

    use mockito;

    use super::*;

    #[test]
    // Asserts that URLs with a scheme other that http return an error
    fn test_cli_get_state_with_prefix_scheme() {
        assert!(get_state_with_prefix("https://test.com", "test").is_err());
        assert!(get_state_with_prefix("file://test", "test").is_err());
    }

    #[test]
    // Asserts that get_state_with_prefix() returns data as expected
    fn test_cli_get_state_with_prefix() {
        let url = mockito::server_url();
        let _m1 = mockito::mock("GET", "/state?address=test")
            .with_body("{\"data\":[{\"address\": \"abc\", \"data\": \"def\"}]}")
            .create();
        let expected = vec![StateEntry {
            address: "abc".to_string(),
            data: "def".to_string(),
        }];
        let result = get_state_with_prefix(&url, "test");

        assert_eq!(result.unwrap(), expected);
    }
}
