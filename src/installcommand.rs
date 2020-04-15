use std::process::{Command as ProcessCommand, Stdio};

use crate::command::Command;
use crate::helmruntime::HelmRuntime;
use clap::ArgMatches;

pub(crate) struct InstallCommand<'a> {
    helm_runtime: &'a mut HelmRuntime,
}

impl<'a> InstallCommand<'a> {
    pub(crate) fn new(execute_helm_command: &'a mut HelmRuntime) -> InstallCommand {
        InstallCommand {
            helm_runtime: execute_helm_command,
        }
    }
}

impl<'a> Command<'a> for InstallCommand<'a> {
    fn get_helm_runtime(&mut self) -> &mut HelmRuntime {
        self.helm_runtime
    }

    fn execute(&mut self, matches: &ArgMatches, commandline: &Option<&str>, helm_home_dir: String) {
        if let Some(command) = commandline {
            if let Some(install_command) = matches.subcommand_matches(command) {
                let mut helm_command = ProcessCommand::new(format!("{}/helm", helm_home_dir));
                helm_command
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .arg(command);

                self.get_helm_runtime()
                    .get_and_set_chart_name(install_command, &mut helm_command);

                if let Some(release) = install_command.value_of("name") {
                    helm_command.args(&["--name", release]);
                    // add global variable key/value 'release.name'
                    self.get_helm_runtime()
                        .set_implicit_var("release.name".to_string(), release.to_string());
                }

                self.get_helm_runtime().apply_common_args(
                    matches,
                    install_command,
                    &mut helm_command,
                );

                self.get_helm_runtime().execute_helm(&mut helm_command)
            }
        }
    }
}
