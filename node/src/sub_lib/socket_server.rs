// Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.
use masq_lib::command::StdStreams;
use std::marker::Send;
use tokio::prelude::Future;

pub trait SocketServer<C>: Send + Future<Item = (), Error = ()> {
    fn get_configuration(&self) -> &C;
    fn initialize_as_privileged(&mut self, args: &[String], streams: &mut StdStreams);
    fn initialize_as_unprivileged(&mut self, args: &[String], streams: &mut StdStreams<'_>);
}
