use std::process::{Command as ProcessCommand, Stdio};

use crate::command::Command;
use crate::executehelm::HelmRuntime;
use clap::ArgMatches;
use std::collections::HashMap;

type GlobalVariableMap<'a> = HashMap<String, String>;
type GlobalVariableRawMap<'a> = HashMap<String, String>;

pub(crate) struct UpgradeCommand<'a> {
    helm_runtime: &'a HelmRuntime,
}

impl<'a> UpgradeCommand<'a> {
    pub(crate) fn new(execute_helm_command: &'a HelmRuntime) -> UpgradeCommand {
        UpgradeCommand {
            helm_runtime: execute_helm_command,
        }
    }
}

impl<'a> Command<'a> for UpgradeCommand<'a> {
    fn get_helm_runtime(&self) -> &HelmRuntime {
        self.helm_runtime
    }

    fn run(&self, matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String) {
        let mut variable_formatted: GlobalVariableMap = GlobalVariableMap::new();
        let mut variable_raw: GlobalVariableRawMap = GlobalVariableRawMap::new();
        if let Some(upgrade_command) = matches.subcommand_matches(command.unwrap()) {
            let mut helm_command = ProcessCommand::new(format!("{}/helm", helm_home_dir));
            helm_command
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .arg(command.unwrap());

            if upgrade_command.is_present("RELEASE") {
                helm_command.arg(upgrade_command.value_of("RELEASE").unwrap());
                // add global variable key/value 'release.name'
                variable_raw.insert(
                    "release.name".to_string(),
                    upgrade_command.value_of("RELEASE").unwrap().to_string(),
                );
            }
            self.get_helm_runtime().get_and_set_chart_name(
                &mut variable_raw,
                upgrade_command,
                &mut helm_command,
            );

            if upgrade_command.is_present("force") {
                helm_command.arg("--force");
            }

            self.get_helm_runtime().apply_common_args(
                matches,
                upgrade_command,
                &mut helm_command,
                &mut variable_formatted,
                &mut variable_raw,
            );

            self.get_helm_runtime().execute_helm(&mut helm_command)
        }
    }
}
