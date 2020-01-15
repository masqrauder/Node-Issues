// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.
pub mod command;
pub mod fake_stream_holder;
pub mod ui_connection;
pub mod ui_gateway;
pub mod ui_traffic_converter;
pub mod utils;

#[macro_use]
pub mod messages;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
