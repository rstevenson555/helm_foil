use std::process::{Command as ProcessCommand, Stdio};

use crate::command::Command;
use crate::helmruntime::HelmRuntime;
use clap::ArgMatches;
use std::collections::HashMap;

pub(crate) struct UpgradeCommand<'a> {
    helm_runtime: &'a mut HelmRuntime,
}

impl<'a> UpgradeCommand<'a> {
    pub(crate) fn new(execute_helm_command: &'a mut HelmRuntime) -> UpgradeCommand {
        UpgradeCommand {
            helm_runtime: execute_helm_command,
        }
    }
}

impl<'a> Command<'a> for UpgradeCommand<'a> {
    fn get_helm_runtime(&mut self) -> &mut HelmRuntime {
        self.helm_runtime
    }

    fn run(&mut self, matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String) {
        if let Some(upgrade_command) = matches.subcommand_matches(command.unwrap()) {
            let mut helm_command = ProcessCommand::new(format!("{}/helm", helm_home_dir));
            helm_command
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .arg(command.unwrap());

            if upgrade_command.is_present("RELEASE") {
                helm_command.arg(upgrade_command.value_of("RELEASE").unwrap());
                // add global variable key/value 'release.name'
                self.get_helm_runtime().set_implicit_var(
                    "release.name".to_string(),
                    upgrade_command.value_of("RELEASE").unwrap().to_string(),
                );
            }
            self.get_helm_runtime()
                .get_and_set_chart_name(upgrade_command, &mut helm_command);

            if upgrade_command.is_present("force") {
                helm_command.arg("--force");
            }

            self.get_helm_runtime()
                .apply_common_args(matches, upgrade_command, &mut helm_command);

            self.get_helm_runtime().execute_helm(&mut helm_command)
        }
    }
}
