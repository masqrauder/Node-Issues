// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.
pub mod command;
pub mod fake_stream_holder;
pub mod messages;
pub mod ui_gateway;
pub mod ui_traffic_converter;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
