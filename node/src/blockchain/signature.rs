// Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(remote = "ethsign::Signature")]
pub struct SerializableSignature {
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
}

impl SerializableSignature {}
