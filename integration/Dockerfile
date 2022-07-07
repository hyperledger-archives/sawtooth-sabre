# Copyright 2018 Cargill Incorporated
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

FROM ubuntu:jammy

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
 && apt-get install -y -q \
    build-essential \
    git \
    unzip \
    libffi-dev \
    libssl-dev \
    libzmq3-dev \
    python3-pip

RUN apt-get update && apt-get install -y -q --no-install-recommends \
    curl \
 && curl -s -S -o /tmp/setup-node.sh https://deb.nodesource.com/setup_6.x

RUN curl https://sh.rustup.rs -sSf > /usr/bin/rustup-init \
 &&  chmod +x /usr/bin/rustup-init \
 && rustup-init -y

 # For Building Protobufs
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
 && TARGET_ARCH=$(dpkg --print-architecture) \
 && if [[ $TARGET_ARCH == "arm64" ]]; then \
      PROTOC_ARCH="aarch_64"; \
    elif [[ $TARGET_ARCH == "amd64" ]]; then \
      PROTOC_ARCH="x86_64"; \
    fi \
 && curl -OLsS https://github.com/google/protobuf/releases/download/v3.20.0/protoc-3.20.0-linux-$PROTOC_ARCH.zip \
 && unzip -o protoc-3.20.0-linux-$PROTOC_ARCH.zip -d /usr/local \
 && rm protoc-3.20.0-linux-$PROTOC_ARCH.zip

ENV PATH=$PATH:/protoc3/bin:/root/.cargo/bin \
 CARGO_INCREMENTAL=0

RUN  rustup target add wasm32-unknown-unknown

WORKDIR /project/example/intkey_multiply/processor
