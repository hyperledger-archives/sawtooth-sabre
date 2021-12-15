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

#[macro_use]
extern crate clap;

use log::LevelFilter;

use sawtooth_sabre::handler::SabreTransactionHandler;
use sawtooth_sdk::processor::TransactionProcessor;

fn main() {
    let matches = clap_app!(wasm_store_tp =>
        (version: crate_version!())
        (about: "Implements the Sawtooth Sabre transaction family")
        (@arg connect: -C --connect +takes_value
         "connection endpoint for validator")
        (@arg verbose: -v --verbose +multiple
         "increase output verbosity"))
    .get_matches();
    let logger = simple_logger::SimpleLogger::new()
        // Switch to UTC timestamps, as local timestamps are not stable, by default. They are only
        // available if the compiler flag "unsound_local_offset" has been set.
        .with_utc_timestamps();
    let logger = match matches.occurrences_of("verbose") {
        0 => logger.with_level(LevelFilter::Warn),
        1 => logger.with_level(LevelFilter::Info),
        2 => logger.with_level(LevelFilter::Debug),
        _ => logger.with_level(LevelFilter::Trace),
    };

    logger.init().expect("Failed to create logger");

    let connect = matches
        .value_of("connect")
        .unwrap_or("tcp://localhost:4004");

    let handler = SabreTransactionHandler::new();
    let mut processor = TransactionProcessor::new(connect);

    processor.add_handler(&handler);
    processor.start();
}
