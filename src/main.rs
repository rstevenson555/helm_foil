use std::env;

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, ArgMatches,
    SubCommand,
};

use command::Command;
use helmruntime::HelmRuntime;
use installcommand::InstallCommand;
use upgradecommand::UpgradeCommand;

mod command;
mod helmruntime;
mod installcommand;
mod upgradecommand;

#[derive(Debug, Clone, Default)]
pub(crate) struct Main {}

impl Main {
    pub(crate) fn parse_command_line<'a>(self: &Main) -> ArgMatches<'a> {
        app_from_crate!()
            // helm install subcommand
            .subcommand(
                SubCommand::with_name("install")
                    .about("install a new application")
                    .arg(
                        Arg::with_name("CHART")
                            .required(true)
                            .takes_value(true)
                            .help("directory location of the chart"),
                    )
                    .arg(
                        Arg::with_name("name")
                            .takes_value(true)
                            .long("name")
                            .short("n"),
                    )
                    .arg(
                        Arg::with_name("valueFiles")
                            .takes_value(true)
                            .multiple(true)
                            .long("values")
                            .short("f"),
                    )
                    .arg(
                        Arg::with_name("set")
                            .multiple(true)
                            .long("set")
                            .takes_value(true)
                            .help("set a variable override"),
                    ),
            )
            // helm upgrade subcommand
            .subcommand(
                SubCommand::with_name("upgrade")
                    .about("upgrade a application")
                    .arg(
                        Arg::with_name("RELEASE")
                            .required(true)
                            .takes_value(true)
                            .help("name the deployment with this value"),
                    )
                    .arg(
                        Arg::with_name("CHART")
                            .required(true)
                            .takes_value(true)
                            .default_value("")
                            .help("directory location of the chart"),
                    )
                    .arg(
                        Arg::with_name("valueFiles")
                            .takes_value(true)
                            .multiple(true)
                            .long("values")
                            .short("f"),
                    )
                    .arg(
                        Arg::with_name("force")
                            .long("force")
                            .help("Force the installation"),
                    )
                    .arg(
                        Arg::with_name("set")
                            .multiple(true)
                            .long("set")
                            .takes_value(true)
                            .help("set a variable override"),
                    ),
            ) // now set global options
            .arg(
                Arg::with_name("tiller-namespace")
                    .long("tiller-namespace")
                    .global(true)
                    .help("Specify the namespace to look for tiller")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("namespace")
                    .long("namespace")
                    .global(true)
                    .help("Specify the namespace")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("timeout")
                    .long("timeout")
                    .global(true)
                    .help("Specify the timeout")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("debug")
                    .long("debug")
                    .global(true)
                    .help("Specify debug mode"),
            )
            .get_matches()
    }
    pub(crate) fn new() -> Main {
        Main {}
    }
}

// static dispatch, as a generic method
fn func<'a, T: Command<'a>>(command: &'a T) {
    println!("do this, static dispatch");
}

fn myfunc(command: &dyn Command) {
    println!("do this dynamic dispatch");
}

fn main() {
    let helm_home_dir;
    if let Ok(homedir) = env::var("HELM_HOME") {
        helm_home_dir = homedir;
    } else {
        panic!("Missing HELM_HOME environment variable");
    }

    let main: Main = Main::new();
    let matches: ArgMatches = main.parse_command_line();

    let helm_runtime = HelmRuntime::new();
    match matches.subcommand_name() {
        Some("install") => {
            let command = InstallCommand::new(&helm_runtime);
            command.run(&matches, &matches.subcommand_name(), helm_home_dir);
        }
        Some("upgrade") => {
            let command = UpgradeCommand::new(&helm_runtime);
            command.run(&matches, &matches.subcommand_name(), helm_home_dir);
        }
        _ => panic!("Unknown command, only install/upgrade are currently supported"),
    }
    //    match matches.subcommand_name() {
    //        Some("install") => {
    //            let insert_command: dyn Command = InstallCommand::new(&execute_helm_commands);
    //            execute_helm_commands.run(&insert_command);
    //        }
    //        //        Some("upgrade") => handle_install(&matches, &matches.subcommand_name(), helm_home_dir),
    //        Some("upgrade") => {
    //            let upgrade_command: dyn Command = UpgradeCommand::new(&execute_helm_commands);
    //            execute_helm_commands.run(&upgrade_command);
    //        }
    //        _ => {}
    //    }

    //    match matches.subcommand_name() {
    //        Some("upgrade") => handle_upgrade(&matches, &matches.subcommand_name(), helm_home_dir),
    //        Some("install") => handle_install(&matches, &matches.subcommand_name(), helm_home_dir),
    //        _ => {}
    //    }
    //    let pattern = format!("(?i)\\{{\\{{\\s*{}\\s*\\}}\\}}", ".Release.Name");
    //    println!("pattern is: {}", pattern);
    //    let release_name = Regex::new(pattern.as_str()).unwrap();
    //    let result: &mut String = &mut "".to_string();
    //
    //    *result = release_name
    //        .replace_all(
    //            "now is the time {{    .release.Name   }} for all good men",
    //            "master-6kdefg",
    //        )
    //        .into_owned();
    //    println!("result {}", result);
}
