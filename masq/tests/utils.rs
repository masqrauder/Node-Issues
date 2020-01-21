// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UiTrafficConverter;
use websocket::sync::Server;
use std::net::{SocketAddr, TcpListener};
use masq_lib::utils::localhost;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::thread;
use websocket::OwnedMessage;
use websocket::server::WsServer;
use websocket::server::NoTlsAcceptor;
use websocket::result::WebSocketError;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::process::{Command, Child, Stdio};

pub struct MasqProcess {}

impl MasqProcess {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start_noninteractive (self, params: Vec<&str>) -> MasqProcessStopHandle {
        #[cfg(not(target_os = "windows"))]
        let executable_name = "masq";
        #[cfg(target_os = "windows")]
        let executable_name = "masq.exe";
        let executable_path = std::env::current_dir().unwrap().join ("target").join ("release").join(executable_name);
        let mut command = Command::new(executable_path);
        let command = command.args(params);
        let child = command.stdout (Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        MasqProcessStopHandle {child}
    }
}

pub struct MasqProcessStopHandle {
    child: Child
}

impl MasqProcessStopHandle {
    pub fn stop (self) -> (String, String, i32) {
        let output = self.child.wait_with_output ();
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap();
                (stdout, stderr, exit_code)
            },
            Err(e) => {
                eprintln! ("Couldn't get output from masq: {:?}", e);
                (String::new(), String::new(), -1)
            },
        }
    }
}
