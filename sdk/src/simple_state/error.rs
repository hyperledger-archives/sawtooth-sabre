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

use std::error::Error as StdError;

use crate::WasmSdkError;

use crate::protos::ProtoConversionError;

#[derive(Debug)]
pub enum SimpleStateError {
    AddresserError(String),
    ProtoConversionError(ProtoConversionError),
    ProtocolBuildError(Box<dyn StdError>),
    SdkError(WasmSdkError),
}

impl std::fmt::Display for SimpleStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            SimpleStateError::AddresserError(ref s) => write!(f, "AddresserError: {}", s),
            SimpleStateError::ProtoConversionError(ref err) => {
                write!(f, "ProtoConversionError: {}", err.description())
            }
            SimpleStateError::ProtocolBuildError(ref err) => {
                write!(f, "ProtocolBuildError: {}", err.description())
            }
            SimpleStateError::SdkError(ref err) => write!(f, "WasmSdkError: {}", err.to_string()),
        }
    }
}

impl From<ProtoConversionError> for SimpleStateError {
    fn from(e: ProtoConversionError) -> Self {
        SimpleStateError::ProtoConversionError(e)
    }
}

impl From<WasmSdkError> for SimpleStateError {
    fn from(e: WasmSdkError) -> Self {
        SimpleStateError::SdkError(e)
    }
}
