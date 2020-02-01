// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use std::fmt::Debug;
use crate::command_context::{CommandContext};
use std::collections::HashMap;
use masq_lib::messages::{FromMessageBody, ToMessageBody, UiSetup, UiSetupValue};
use masq_lib::ui_gateway::NodeFromUiMessage;

#[derive (Debug, PartialEq)]
pub enum CommandError {
    Transaction(String),
}

pub trait Command: Debug {
    fn execute(&self, context: &mut dyn CommandContext) -> Result<(), CommandError>;
}

#[derive (Debug, PartialEq)]
pub struct SetupCommand {
    pub values: HashMap<String, String>,
}

impl Command for SetupCommand {
    fn execute(&self, context: &mut dyn CommandContext) -> Result<(), CommandError> {
        let mut values: Vec<UiSetupValue> = self.values.iter()
            .map (|(name, value)| UiSetupValue::new (name, value))
            .collect();
        values.sort_by(|a, b| a.name.partial_cmp(&b.name).expect("String comparison failed"));
        let ntum = match context.transact (NodeFromUiMessage {
            client_id: 0,
            body: UiSetup {
                values
            }.tmb(0)
        }) {
            Ok(ntum) => ntum,
            Err(e) => return Err(CommandError::Transaction(e)),
        };
        let mut response: UiSetup = match UiSetup::fmb (ntum.body) {
            Ok ((r, _)) => r,
            Err (ume) => return Err(CommandError::Transaction(format!("{:?}", ume))),
        };
        response.values
            .sort_by(|a, b| a.name.partial_cmp(&b.name).expect("String comparison failed"));
        write!(context.stdout(), "NAME                      VALUE\n").expect ("write! failed");
        response.values.into_iter()
            .for_each (|value| write!(context.stdout(), "{:26}{}\n", value.name, value.value).expect ("write! failed"));
        Ok(())
    }
}

impl SetupCommand {
    pub fn new (pieces: Vec<String>) -> Self {
        let values = pieces.into_iter()
            .skip(1)
            .map(|attr| {
                let pair: Vec<&str> = attr.split ("=").collect();
                (pair[0].to_string(), pair[1].to_string())
            })
            .collect::<HashMap<String, String>> ();
        Self {
            values
        }
    }

    pub fn validator (value: String) -> Result<(), String> {
        if value.starts_with ("=") || value.ends_with ("=") || !value.contains ("=") {
            Err(format!("Attribute syntax: <name>=<value>, not {}", value))
        }
        else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mocks::CommandContextMock;
    use std::sync::{Mutex, Arc};
    use masq_lib::ui_gateway::{NodeToUiMessage, NodeFromUiMessage};
    use masq_lib::ui_gateway::MessageTarget::ClientId;

    #[test]
    fn setup_command_validator_rejects_arg_without_equals() {
        let result = SetupCommand::validator ("noequals".to_string());

        assert_eq! (result, Err("Attribute syntax: <name>=<value>, not noequals".to_string()));
    }

    #[test]
    fn setup_command_validator_rejects_arg_with_initial_equals() {
        let result = SetupCommand::validator ("=initialequals".to_string());

        assert_eq! (result, Err("Attribute syntax: <name>=<value>, not =initialequals".to_string()));
    }

    #[test]
    fn setup_command_validator_rejects_arg_with_terminal_equals() {
        let result = SetupCommand::validator ("terminalequals=".to_string());

        assert_eq! (result, Err("Attribute syntax: <name>=<value>, not terminalequals=".to_string()));
    }

    #[test]
    fn setup_command_validator_accepts_valid_attribute() {
        let result = SetupCommand::validator ("central=equals".to_string());

        assert_eq! (result, Ok(()));
    }

    #[test]
    fn setup_command_happy_path () {
        let transact_params_arc = Arc::new (Mutex::new (vec![]));
        let mut context = CommandContextMock::new()
            .transact_params(&transact_params_arc)
            .transact_result (Ok (NodeToUiMessage {
                target: ClientId(0),
                body: UiSetup {
                    values: vec![
                        UiSetupValue::new ("c", "3"),
                        UiSetupValue::new ("dddd", "4444"),
                    ]
                }.tmb (0)
            }));
        let stdout_arc = context.stdout_arc();
        let stderr_arc = context.stderr_arc();
        let subject = SetupCommand::new (vec!["setup".to_string(), "a=1".to_string(), "bbbb=2222".to_string()]);

        let result = subject.execute (&mut context);

        assert_eq! (result, Ok(()));
        let transact_params = transact_params_arc.lock().unwrap();
        assert_eq! (*transact_params, vec![NodeFromUiMessage {
            client_id: 0,
            body: UiSetup {
                values: vec![
                    UiSetupValue::new ("a", "1"),
                    UiSetupValue::new ("bbbb", "2222"),
                ]
            }.tmb(0)
        }]);
        assert_eq! (stdout_arc.lock().unwrap().get_string(),
            "NAME                      VALUE\nc                         3\ndddd                      4444\n");
        assert_eq! (stderr_arc.lock().unwrap().get_string(), String::new());
    }
}
