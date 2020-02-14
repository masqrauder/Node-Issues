// Copyright (c) 2019, MASQ (https://masq.ai). All rights reserved.

use crate::daemon::launch_verifier::LaunchVerification::{
    CleanFailure, DirtyFailure, InterventionRequired, Launched,
};
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessExt, ProcessStatus, Signal, SystemExt};
use websocket::ClientBuilder;

// Note: if the INTERVALs are half the DELAYs or greater, the tests below will need to change,
// because they depend on being able to fail twice and still succeed.
const DELAY_FOR_RESPONSE_MS: u64 = 1000;
const RESPONSE_CHECK_INTERVAL_MS: u64 = 250;
const DELAY_FOR_DEATH_MS: u64 = 1000;
const DEATH_CHECK_INTERVAL_MS: u64 = 250;

pub trait VerifierTools {
    fn can_connect_to_ui_gateway(&self, ui_port: u16) -> bool;
    fn process_is_running(&self, process_id: u32) -> bool;
    fn kill_process(&self, process_id: u32);
    fn delay(&self, milliseconds: u64);
}

#[derive(Default)]
pub struct VerifierToolsReal {}

impl VerifierTools for VerifierToolsReal {
    fn can_connect_to_ui_gateway(&self, ui_port: u16) -> bool {
        let mut builder = match ClientBuilder::new(format!("ws://127.0.0.1:{}", ui_port).as_str()) {
            Ok(builder) => builder.add_protocol("MASQNode-UIv2"),
            Err(e) => panic!(format!("{:?}", e)),
        };
        builder.connect_insecure().is_ok()
    }

    fn process_is_running(&self, process_id: u32) -> bool {
        match Self::system_with_process(process_id).get_process(Self::convert_pid(process_id)) {
            None => false,
            Some(process) => Self::is_alive(process.status()),
        }
    }

    fn kill_process(&self, process_id: u32) {
        if let Some(process) =
            Self::system_with_process(process_id).get_process(Self::convert_pid(process_id))
        {
            process.kill(Signal::Kill);
        }
    }

    fn delay(&self, milliseconds: u64) {
        thread::sleep(Duration::from_millis(milliseconds));
    }
}

impl VerifierToolsReal {
    pub fn new() -> Self {
        Self {}
    }

    fn system_with_process(process_id: u32) -> sysinfo::System {
        let process_id = Self::convert_pid(process_id);
        let mut system: sysinfo::System = sysinfo::SystemExt::new();
        system.refresh_process(process_id);
        system
    }

    #[cfg(not(target_os = "windows"))]
    fn convert_pid(process_id: u32) -> i32 {
        process_id as i32
    }

    #[cfg(target_os = "windows")]
    fn convert_pid(process_id: u32) -> usize {
        process_id as usize
    }

    #[cfg(target_os = "linux")]
    fn is_alive(process_status: ProcessStatus) -> bool {
        match process_status {
            ProcessStatus::Dead => false,
            ProcessStatus::Zombie => false,
            _ => true,
        }
    }

    #[cfg(target_os = "macos")]
    fn is_alive(process_status: ProcessStatus) -> bool {
        match process_status {
            ProcessStatus::Zombie => false,
            ProcessStatus::Unknown(0) => false, // This value was observed in practice; its meaning is unclear.
            _ => true,
        }
    }

    #[cfg(target_os = "windows")]
    fn is_alive(process_status: ProcessStatus) -> bool {
        match process_status {
            ProcessStatus::Run => true,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LaunchVerification {
    Launched,             // Responded to contact via UiGateway
    CleanFailure,         // No response from UiGateway, no process at process_id
    DirtyFailure,         // No response from UiGateway, process at process_id, killed, disappeared
    InterventionRequired, // No response from UiGateway, process at process_id, killed, still there
}

pub trait LaunchVerifier {
    fn verify_launch(&self, process_id: u32, ui_port: u16) -> LaunchVerification;
}

pub struct LaunchVerifierReal {
    verifier_tools: Box<dyn VerifierTools>,
}

impl Default for LaunchVerifierReal {
    fn default() -> Self {
        LaunchVerifierReal {
            verifier_tools: Box::new(VerifierToolsReal::new()),
        }
    }
}

impl LaunchVerifier for LaunchVerifierReal {
    fn verify_launch(&self, process_id: u32, ui_port: u16) -> LaunchVerification {
        if self.await_ui_connection(ui_port) {
            Launched
        } else if self.verifier_tools.process_is_running(process_id) {
            self.verifier_tools.kill_process(process_id);
            if self.await_process_death(process_id) {
                DirtyFailure
            } else {
                InterventionRequired
            }
        } else {
            CleanFailure
        }
    }
}

impl LaunchVerifierReal {
    pub fn new() -> Self {
        Self::default()
    }

    fn await_ui_connection(&self, ui_port: u16) -> bool {
        let mut accumulated_delay = 0;
        loop {
            if self.verifier_tools.can_connect_to_ui_gateway(ui_port) {
                return true;
            }
            if accumulated_delay > DELAY_FOR_RESPONSE_MS {
                return false;
            }
            self.verifier_tools.delay(RESPONSE_CHECK_INTERVAL_MS);
            accumulated_delay += RESPONSE_CHECK_INTERVAL_MS;
        }
    }

    fn await_process_death(&self, pid: u32) -> bool {
        let mut accumulated_delay = 0;
        loop {
            self.verifier_tools.delay(DEATH_CHECK_INTERVAL_MS);
            accumulated_delay += DEATH_CHECK_INTERVAL_MS;
            if accumulated_delay > DELAY_FOR_DEATH_MS {
                return false;
            }
            if !self.verifier_tools.process_is_running(pid) {
                return true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::launch_verifier::LaunchVerification::{
        CleanFailure, InterventionRequired, Launched,
    };
    use crate::sub_lib::utils::localhost;
    use crate::test_utils::find_free_port;
    use std::cell::RefCell;
    use std::net::SocketAddr;
    use std::process::{Child, Command};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use websocket::server::sync::Server;

    struct VerifierToolsMock {
        can_connect_to_ui_gateway_params: Arc<Mutex<Vec<u16>>>,
        can_connect_to_ui_gateway_results: RefCell<Vec<bool>>,
        process_is_running_params: Arc<Mutex<Vec<u32>>>,
        process_is_running_results: RefCell<Vec<bool>>,
        kill_process_params: Arc<Mutex<Vec<u32>>>,
        delay_params: Arc<Mutex<Vec<u64>>>,
    }

    impl VerifierTools for VerifierToolsMock {
        fn can_connect_to_ui_gateway(&self, ui_port: u16) -> bool {
            self.can_connect_to_ui_gateway_params
                .lock()
                .unwrap()
                .push(ui_port);
            self.can_connect_to_ui_gateway_results
                .borrow_mut()
                .remove(0)
        }

        fn process_is_running(&self, process_id: u32) -> bool {
            self.process_is_running_params
                .lock()
                .unwrap()
                .push(process_id);
            self.process_is_running_results.borrow_mut().remove(0)
        }

        fn kill_process(&self, process_id: u32) {
            self.kill_process_params.lock().unwrap().push(process_id);
        }

        fn delay(&self, milliseconds: u64) {
            self.delay_params.lock().unwrap().push(milliseconds);
        }
    }

    impl VerifierToolsMock {
        fn new() -> Self {
            VerifierToolsMock {
                can_connect_to_ui_gateway_params: Arc::new(Mutex::new(vec![])),
                can_connect_to_ui_gateway_results: RefCell::new(vec![]),
                process_is_running_params: Arc::new(Mutex::new(vec![])),
                process_is_running_results: RefCell::new(vec![]),
                kill_process_params: Arc::new(Mutex::new(vec![])),
                delay_params: Arc::new(Mutex::new(vec![])),
            }
        }

        fn can_connect_to_ui_gateway_params(mut self, params: &Arc<Mutex<Vec<u16>>>) -> Self {
            self.can_connect_to_ui_gateway_params = params.clone();
            self
        }

        fn can_connect_to_ui_gateway_result(self, result: bool) -> Self {
            self.can_connect_to_ui_gateway_results
                .borrow_mut()
                .push(result);
            self
        }

        fn process_is_running_params(mut self, params: &Arc<Mutex<Vec<u32>>>) -> Self {
            self.process_is_running_params = params.clone();
            self
        }

        fn process_is_running_result(self, result: bool) -> Self {
            self.process_is_running_results.borrow_mut().push(result);
            self
        }

        fn kill_process_params(mut self, params: &Arc<Mutex<Vec<u32>>>) -> Self {
            self.kill_process_params = params.clone();
            self
        }

        fn delay_params(mut self, params: &Arc<Mutex<Vec<u64>>>) -> Self {
            self.delay_params = params.clone();
            self
        }
    }

    #[test]
    fn detects_successful_launch_after_two_attempts() {
        let can_connect_to_ui_gateway_params_arc = Arc::new(Mutex::new(vec![]));
        let delay_parms_arc = Arc::new(Mutex::new(vec![]));
        let tools = VerifierToolsMock::new()
            .can_connect_to_ui_gateway_params(&can_connect_to_ui_gateway_params_arc)
            .delay_params(&delay_parms_arc)
            .can_connect_to_ui_gateway_result(false)
            .can_connect_to_ui_gateway_result(false)
            .can_connect_to_ui_gateway_result(true);
        let mut subject = LaunchVerifierReal::new();
        subject.verifier_tools = Box::new(tools);

        let result = subject.verify_launch(1234, 4321);

        assert_eq!(result, Launched);
        let can_connect_to_ui_gateway_parms = can_connect_to_ui_gateway_params_arc.lock().unwrap();
        assert_eq!(*can_connect_to_ui_gateway_parms, vec![4321, 4321, 4321]);
        let delay_params = delay_parms_arc.lock().unwrap();
        assert_eq!(
            *delay_params,
            vec![RESPONSE_CHECK_INTERVAL_MS, RESPONSE_CHECK_INTERVAL_MS,]
        );
    }

    #[test]
    fn detects_clean_failure() {
        let connect_failure_count = (DELAY_FOR_RESPONSE_MS / RESPONSE_CHECK_INTERVAL_MS) + 1;
        let delay_params_arc = Arc::new(Mutex::new(vec![]));
        let process_is_running_params_arc = Arc::new(Mutex::new(vec![]));
        let mut tools = VerifierToolsMock::new()
            .delay_params(&delay_params_arc)
            .process_is_running_params(&process_is_running_params_arc)
            .can_connect_to_ui_gateway_result(false);
        for _ in 0..connect_failure_count {
            tools = tools.can_connect_to_ui_gateway_result(false);
        }
        tools = tools.process_is_running_result(false);
        let mut subject = LaunchVerifierReal::new();
        subject.verifier_tools = Box::new(tools);

        let result = subject.verify_launch(1234, 4321);

        assert_eq!(result, CleanFailure);
        let delay_params = delay_params_arc.lock().unwrap();
        assert_eq!(delay_params.len() as u64, connect_failure_count);
        delay_params
            .iter()
            .for_each(|delay| assert_eq!(delay, &RESPONSE_CHECK_INTERVAL_MS));
        let process_is_running_params = process_is_running_params_arc.lock().unwrap();
        assert_eq!(*process_is_running_params, vec![1234]);
    }

    #[test]
    fn detects_dirty_failure_after_two_attempts() {
        let connect_failure_count = (DELAY_FOR_RESPONSE_MS / RESPONSE_CHECK_INTERVAL_MS) + 1;
        let delay_params_arc = Arc::new(Mutex::new(vec![]));
        let kill_process_params_arc = Arc::new(Mutex::new(vec![]));
        let process_is_running_params_arc = Arc::new(Mutex::new(vec![]));
        let mut tools = VerifierToolsMock::new()
            .delay_params(&delay_params_arc)
            .process_is_running_params(&process_is_running_params_arc)
            .kill_process_params(&kill_process_params_arc)
            .can_connect_to_ui_gateway_result(false);
        for _ in 0..connect_failure_count {
            tools = tools.can_connect_to_ui_gateway_result(false);
        }
        tools = tools
            .process_is_running_result(true)
            .process_is_running_result(true)
            .process_is_running_result(false);
        let mut subject = LaunchVerifierReal::new();
        subject.verifier_tools = Box::new(tools);

        let result = subject.verify_launch(1234, 4321);

        assert_eq!(result, DirtyFailure);
        let delay_params = delay_params_arc.lock().unwrap();
        assert_eq!(delay_params.len() as u64, connect_failure_count + 2);
        delay_params
            .iter()
            .for_each(|delay| assert_eq!(delay, &RESPONSE_CHECK_INTERVAL_MS));
        let kill_process_params = kill_process_params_arc.lock().unwrap();
        assert_eq!(*kill_process_params, vec![1234]);
        let process_is_running_params = process_is_running_params_arc.lock().unwrap();
        assert_eq!(*process_is_running_params, vec![1234, 1234, 1234]);
    }

    #[test]
    fn detects_intervention_required_after_two_attempts() {
        let connect_failure_count = (DELAY_FOR_RESPONSE_MS / RESPONSE_CHECK_INTERVAL_MS) + 1;
        let death_check_count = (DELAY_FOR_DEATH_MS / DEATH_CHECK_INTERVAL_MS) + 1;
        let delay_params_arc = Arc::new(Mutex::new(vec![]));
        let kill_process_params_arc = Arc::new(Mutex::new(vec![]));
        let process_is_running_params_arc = Arc::new(Mutex::new(vec![]));
        let mut tools = VerifierToolsMock::new()
            .delay_params(&delay_params_arc)
            .process_is_running_params(&process_is_running_params_arc)
            .kill_process_params(&kill_process_params_arc)
            .can_connect_to_ui_gateway_result(false);
        for _ in 0..connect_failure_count {
            tools = tools.can_connect_to_ui_gateway_result(false);
        }
        for _ in 0..death_check_count {
            tools = tools.process_is_running_result(true);
        }
        let mut subject = LaunchVerifierReal::new();
        subject.verifier_tools = Box::new(tools);

        let result = subject.verify_launch(1234, 4321);

        assert_eq!(result, InterventionRequired);
        let delay_params = delay_params_arc.lock().unwrap();
        assert_eq!(
            delay_params.len() as u64,
            connect_failure_count + death_check_count
        );
        delay_params
            .iter()
            .for_each(|delay| assert_eq!(delay, &RESPONSE_CHECK_INTERVAL_MS));
        let kill_process_params = kill_process_params_arc.lock().unwrap();
        assert_eq!(*kill_process_params, vec![1234]);
        let process_is_running_params = process_is_running_params_arc.lock().unwrap();
        assert_eq!(process_is_running_params.len() as u64, death_check_count);
        process_is_running_params
            .iter()
            .for_each(|pid| assert_eq!(pid, &1234));
    }

    #[test]
    fn can_connect_to_ui_gateway_handles_success() {
        let port = find_free_port();
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let mut server = Server::bind(SocketAddr::new(localhost(), port)).unwrap();
            tx.send(()).unwrap();
            let upgrade = server.accept().expect("Couldn't accept connection");
            let _ = upgrade.accept().unwrap();
        });
        let subject = VerifierToolsReal::new();
        rx.recv().unwrap();

        let result = subject.can_connect_to_ui_gateway(port);

        assert_eq!(result, true);
    }

    #[test]
    fn can_connect_to_ui_gateway_handles_failure() {
        let port = find_free_port();
        let subject = VerifierToolsReal::new();

        let result = subject.can_connect_to_ui_gateway(port);

        assert_eq!(result, false);
    }

    fn make_long_running_child() -> Child {
        #[cfg(not(target_os = "windows"))]
        let child = Command::new("tail")
            .args(vec!["-f", "/dev/null"])
            .spawn()
            .unwrap();
        #[cfg(target_os = "windows")]
        let child = Command::new("cmd")
            .args(vec!["/c", "pause"])
            .spawn()
            .unwrap();
        child
    }

    #[test]
    fn kill_process_and_process_is_running_work() {
        let subject = VerifierToolsReal::new();
        let child = make_long_running_child();

        let before = subject.process_is_running(child.id());

        subject.kill_process(child.id());

        let after = subject.process_is_running(child.id());

        assert_eq!(before, true);
        assert_eq!(after, false);
    }

    #[test]
    fn delay_works() {
        let subject = VerifierToolsReal::new();
        let begin = Instant::now();

        subject.delay(25);

        let end = Instant::now();
        let interval = end.duration_since(begin).as_millis();
        assert!(interval >= 25);
        assert!(interval < 50);
    }

    #[test]
    fn is_alive_works() {
        #[cfg(target_os = "linux")]
        {
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Idle), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Run), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Sleep), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Stop), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Zombie), false);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Tracing), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Dead), false);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Wakekill), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Waking), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Parked), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Unknown(0)), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Unknown(1)), true);
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Idle), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Run), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Sleep), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Stop), true);
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Zombie), false);
            assert_eq!(
                VerifierToolsReal::is_alive(ProcessStatus::Unknown(0)),
                false
            );
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Unknown(1)), true);
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(VerifierToolsReal::is_alive(ProcessStatus::Run), true);
        }
    }
}
