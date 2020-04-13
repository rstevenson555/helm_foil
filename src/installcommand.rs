use std::process::{Command as ProcessCommand, Stdio};

use crate::command::Command;
use crate::helmruntime::HelmRuntime;
use clap::ArgMatches;
use std::collections::HashMap;

type GlobalVariableMap<'a> = HashMap<String, String>;
type GlobalVariableRawMap<'a> = HashMap<String, String>;

#[derive(Debug, Clone)]
pub(crate) struct InstallCommand<'a> {
    helm_runtime: &'a HelmRuntime,
}

impl<'a> InstallCommand<'a> {
    pub(crate) fn new(execute_helm_command: &'a HelmRuntime) -> InstallCommand {
        InstallCommand {
            helm_runtime: execute_helm_command,
        }
    }
}

impl<'a> Command<'a> for InstallCommand<'a> {
    fn get_helm_runtime(&self) -> &HelmRuntime {
        self.helm_runtime
    }

    fn run(&self, matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String) {
        let mut variable_formatted: GlobalVariableMap = GlobalVariableMap::new();
        let mut variable_raw: GlobalVariableRawMap = GlobalVariableRawMap::new();
        if let Some(install_command) = matches.subcommand_matches(command.unwrap()) {
            let mut helm_command = ProcessCommand::new(format!("{}/helm", helm_home_dir));
            helm_command
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .arg(command.unwrap());

            self.get_helm_runtime().get_and_set_chart_name(
                &mut variable_raw,
                install_command,
                &mut helm_command,
            );

            if install_command.is_present("name") {
                helm_command.args(&["--name", install_command.value_of("name").unwrap()]);
                // add global variable key/value 'release.name'
                variable_raw.insert(
                    "release.name".to_string(),
                    install_command.value_of("name").unwrap().to_string(),
                );
            }

            self.get_helm_runtime().apply_common_args(
                matches,
                install_command,
                &mut helm_command,
                &mut variable_formatted,
                &mut variable_raw,
            );

            self.get_helm_runtime().execute_helm(&mut helm_command)
        }
    }
}
