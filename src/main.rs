use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command as ProcessCommand, Output, Stdio};
use std::{env, fs};

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, ArgMatches,
    SubCommand,
};
use regex::Regex;

fn read_values_file(filename: &str) -> String {
    let error_msg = format!("Something went wrong reading the file {}", filename);
    let estr: &str = error_msg.as_str();

    fs::read_to_string(filename).expect(estr)
}

/*
make the pattern that we are matching against
*/
fn make_pattern(var: &str) -> String {
    format!("(?i)\\{{\\{{\\s*.{}\\s*\\}}\\}}", var)
}

fn replace_implicit_vars(result: &mut String, variable_raw: &GlobalVariableRawMap) {
    let release_name_pattern = Regex::new(make_pattern("Release.Name").as_str()).unwrap();

    if variable_raw.contains_key("release.name") {
        *result = release_name_pattern
            .replace_all(
                result.as_str(),
                variable_raw.get("release.name").unwrap().as_str(),
            )
            .into_owned();
    }
    let chart_name_pattern = Regex::new(make_pattern("Chart.Name").as_str()).unwrap();

    if variable_raw.contains_key("chart.name") {
        *result = chart_name_pattern
            .replace_all(
                result.as_str(),
                variable_raw.get("chart.name").unwrap().as_str(),
            )
            .into_owned();
    }

    let branch_name_pattern = Regex::new(make_pattern("Branch.Name").as_str()).unwrap();
    if variable_raw.contains_key("source.branch") {
        *result = branch_name_pattern
            .replace_all(
                result.as_str(),
                variable_raw.get("source.branch").unwrap().as_str(),
            )
            .into_owned();
    }

    let previous_branch_pattern = Regex::new(make_pattern("Previous.Branch").as_str()).unwrap();
    if variable_raw.contains_key("previous.branch") {
        *result = previous_branch_pattern
            .replace_all(
                result.as_str(),
                variable_raw.get("previous.branch").unwrap().as_str(),
            )
            .into_owned();
    }

    let starting_canary_percentage_pattern =
        Regex::new(make_pattern("Starting.Canary.Percentage").as_str()).unwrap();
    if variable_raw.contains_key("starting.canary.percentage") {
        *result = starting_canary_percentage_pattern
            .replace_all(
                result.as_str(),
                variable_raw
                    .get("starting.canary.percentage")
                    .unwrap()
                    .as_str(),
            )
            .into_owned();
    }
}

fn replace_explicit_vars(
    override_file_result: &mut String,
    pattern_str: &str,
    variable_formatted: &GlobalVariableMap,
) {
    let pattern = Regex::new(pattern_str).unwrap();
    *override_file_result = pattern
        .replace_all(
            override_file_result.as_str(),
            variable_formatted
                .get(pattern_str.to_owned().as_str())
                .unwrap()
                .as_str(),
        )
        .into_owned();
}

fn apply_common_args(
    global_args: &ArgMatches,
    subcommand: &ArgMatches,
    helm_command: &mut Command,
    variable_formatted: &mut GlobalVariableMap,
    variable_raw: &mut GlobalVariableRawMap,
) {
    let config_env_yaml: &mut String = &mut "".to_string();
    let values_yaml: &mut String = &mut "".to_string();

    if variable_raw.contains_key("chart.path") {
        *values_yaml = read_values_file(
            format!("{}/values.yaml", variable_raw.get("chart.path").unwrap()).as_str(),
        );
        // VALUES file
        replace_implicit_vars(values_yaml, variable_raw);
    } else {
        panic!("missing chart specified on the command line");
    }

    let override_filename: &mut String = &mut "".to_string();
    if subcommand.is_present("valueFiles") {
        *override_filename = subcommand.value_of("valueFiles").unwrap().to_string();
        helm_command.args(&["-f", override_filename]);
        *config_env_yaml = read_values_file(override_filename);
    }

    let set_values: Vec<&str>;
    if subcommand.is_present("set") {
        set_values = subcommand.values_of("set").unwrap().collect();
        // loop over all --sets on the command line
        for set_var in set_values.iter() {
            let split_parts: Vec<&str> = set_var.split('=').collect();

            // convert the --set arguments on the command line to global variables formatted like
            // {{.SetKey}}; example image.tag becomes {{.Values.image.tag}} template variable
            let variable_format = make_pattern(format!("Values.{}", split_parts[0]).as_str());
            println!("set template variable {}", variable_format);

            variable_formatted.insert(variable_format.to_owned(), split_parts[1].to_owned());
            variable_raw.insert(split_parts[0].to_owned(), split_parts[1].to_owned());

            // now replace the explicit variables declared from the command line from the -f override file
            // env CONFIG/*.yaml files
            replace_explicit_vars(config_env_yaml, &variable_format, variable_formatted);

            // now replace the explicit variables declared from the command line from the chart/values.yaml file
            // VALUES file
            replace_explicit_vars(values_yaml, &variable_format, variable_formatted);

            helm_command.args(&["--set", (*set_var).to_string().as_str()]);
        }

        // replace values.yaml contents
        replace_implicit_vars(values_yaml, variable_raw);

        if subcommand.is_present("valueFiles") {
            // replace global vars
            // create more implicit variables in config/*.yaml
            replace_implicit_vars(config_env_yaml, variable_raw);
        }

        println!("{}", config_env_yaml); // => "xxxxx xxxxx!"
    }

    if global_args.is_present("tiller-namespace") {
        helm_command.args(&[
            "--tiller-namespace",
            global_args.value_of("tiller-namespace").unwrap(),
        ]);
    }
    if global_args.is_present("namespace") {
        helm_command.args(&["--namespace", global_args.value_of("namespace").unwrap()]);
    }
    if global_args.is_present("timeout") {
        helm_command.args(&["--timeout", global_args.value_of("timeout").unwrap()]);
    }
    if global_args.is_present("debug") {
        helm_command.arg("--debug");
    }

    // write output
    match File::create(override_filename) {
        Ok(mut file) => {
            if let Err(err) = file.write_all(config_env_yaml.as_bytes()) {
                panic!("Error writing file {}", err);
            }
        }
        Err(e) => {
            panic!("Error writing override file {}", e);
        }
    }

    match File::create(format!(
        "{}/values.yaml",
        variable_raw.get("chart.path").unwrap()
    )) {
        Ok(mut file) => {
            if let Err(err) = file.write_all(values_yaml.as_bytes()) {
                panic!("Error writing file {}", err);
            }
        }
        Err(e) => {
            panic!("Error writing values file out {}", e);
        }
    }
}

fn execute_helm(helm_command: &mut Command) {
    println!("about to execute {:?}", helm_command);
    let output: Output = helm_command
        .spawn()
        .expect("failed to spawn helm")
        .wait_with_output()
        .expect("failed to wait on helm to complete");

    if output.status.success() {
        if let Ok(out) = String::from_utf8(output.stdout) {
            println!("[helm] {}", out)
        } else {
            eprintln!("[helm] Error reading stdout");
        }
    } else if let Ok(out) = String::from_utf8(output.stderr) {
        eprintln!("[helm] {}", out);
    } else {
        eprintln!("[helm] Error reading stderr");
    }
}

type GlobalVariableMap<'a> = HashMap<String, String>;
type GlobalVariableRawMap<'a> = HashMap<String, String>;

fn handle_install(matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String) {
    let mut variable_formatted: GlobalVariableMap = GlobalVariableMap::new();
    let mut variable_raw: GlobalVariableRawMap = GlobalVariableRawMap::new();
    if let Some(install_command) = matches.subcommand_matches(command.unwrap()) {
        let mut helm_command = ProcessCommand::new(format!("{}/helm", helm_home_dir));
        helm_command
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .arg(command.unwrap());

        get_and_set_chart_name(&mut variable_raw, install_command, &mut helm_command);

        if install_command.is_present("name") {
            helm_command.args(&["--name", install_command.value_of("name").unwrap()]);
            // add global variable key/value 'release.name'
            variable_raw.insert(
                "release.name".to_string(),
                install_command.value_of("name").unwrap().to_string(),
            );
        }

        apply_common_args(
            matches,
            install_command,
            &mut helm_command,
            &mut variable_formatted,
            &mut variable_raw,
        );

        execute_helm(&mut helm_command)
    }
}

fn handle_upgrade(matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String) {
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
        get_and_set_chart_name(&mut variable_raw, upgrade_command, &mut helm_command);

        if upgrade_command.is_present("force") {
            helm_command.arg("--force");
        }

        apply_common_args(
            matches,
            upgrade_command,
            &mut helm_command,
            &mut variable_formatted,
            &mut variable_raw,
        );

        execute_helm(&mut helm_command)
    }
}

fn get_and_set_chart_name(
    variable_raw: &mut GlobalVariableRawMap,
    upgrade_command: &ArgMatches,
    helm_command: &dyn Command,
) {
    if upgrade_command.is_present("CHART") {
        let chart_path = upgrade_command.value_of("CHART").unwrap();
        helm_command.arg(chart_path);
        let path = Path::new(chart_path);
        // add global variable key/value 'chart.name'
        variable_raw.insert(
            "chart.name".to_string(),
            path.file_name().unwrap().to_str().unwrap().to_string(),
        );
        // add global variable key/value 'chart.path'
        variable_raw.insert("chart.path".to_string(), chart_path.to_string());
    }
}

trait Command {
    fn new(name: &'static str) -> Self;
    // Traits can provide default method definitions.
    fn run(&self) {
        println!("{} says {}", self.name(), self.noise());
    }
}

pub struct UpgradeCommand<'a> {
    helm_runtime: &'a ExecuteHelmCommands,
}

impl<'a> Command for UpgradeCommand<'a> {
    fn new(execute_helm_command: &ExecuteHelmCommands) -> UpgradeCommand {
        UpgradeCommand {
            helm_runtime: execute_helm_command,
        }
    }

    fn run(&self) {
        println!("{} says {}", self.name(), self.noise());
    }
}

impl<'a> UpgradeCommand<'a> {
    //    pub(crate) fn new(execute_helm_command: &ExecuteHelmCommands) -> UpgradeCommand {
    //        UpgradeCommand {
    //            helm_runtime: execute_helm_command,
    //        }
    //    }

    fn handle_upgrade(matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String) {
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
            get_and_set_chart_name(&mut variable_raw, upgrade_command, &mut helm_command);

            if upgrade_command.is_present("force") {
                helm_command.arg("--force");
            }

            apply_common_args(
                matches,
                upgrade_command,
                &mut helm_command,
                &mut variable_formatted,
                &mut variable_raw,
            );

            execute_helm(&mut helm_command)
        }
    }
}

pub struct InstallCommand<'a> {
    helm_runtime: &'a ExecuteHelmCommands,
}

impl<'a> Command for InstallCommand<'a> {
    fn new(execute_helm_command: &ExecuteHelmCommands) -> InstallCommand {
        InstallCommand {
            helm_runtime: execute_helm_command,
        }
    }

    fn run(&self) {
        println!("{} says {}", self.name(), self.noise());
    }
}

impl<'a> InstallCommand<'a> {
    //    pub(crate) fn new(execute_helm_command: &ExecuteHelmCommands) -> InstallCommand {
    //        InstallCommand {
    //            helm_runtime: execute_helm_command,
    //        }
    //    }
    fn handle_install(matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String) {
        let mut variable_formatted: GlobalVariableMap = GlobalVariableMap::new();
        let mut variable_raw: GlobalVariableRawMap = GlobalVariableRawMap::new();
        if let Some(install_command) = matches.subcommand_matches(command.unwrap()) {
            let mut helm_command = ProcessCommand::new(format!("{}/helm", helm_home_dir));
            helm_command
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .arg(command.unwrap());

            get_and_set_chart_name(&mut variable_raw, install_command, &mut helm_command);

            if install_command.is_present("name") {
                helm_command.args(&["--name", install_command.value_of("name").unwrap()]);
                // add global variable key/value 'release.name'
                variable_raw.insert(
                    "release.name".to_string(),
                    install_command.value_of("name").unwrap().to_string(),
                );
            }

            apply_common_args(
                matches,
                install_command,
                &mut helm_command,
                &mut variable_formatted,
                &mut variable_raw,
            );

            execute_helm(&mut helm_command)
        }
    }
}

pub struct ExecuteHelmCommands {
    variable_raw: HashMap<String, String>,
    variable_formatted: HashMap<String, String>,
}

impl ExecuteHelmCommands {
    pub(crate) fn new() -> ExecuteHelmCommands {
        ExecuteHelmCommands {
            variable_raw: HashMap::new(),
            variable_formatted: HashMap::new(),
        }
    }

    //    pub(crate) fn run(&self, command: &Command) -> () {}

    pub(crate) fn get_raw_variables(&self) -> &HashMap<String, String> {
        &self.variable_raw
    }
    pub(crate) fn get_formatted_variables(&self) -> &HashMap<String, String> {
        &self.variable_formatted
    }
}

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

fn main() {
    let helm_home_dir;
    if let Ok(homedir) = env::var("HELM_HOME") {
        helm_home_dir = homedir;
    } else {
        panic!("Missing HELM_HOME environment variable");
    }

    let main: Main = Main::new();
    let matches: ArgMatches = main.parse_command_line();

    let execute_helm_commands = ExecuteHelmCommands::new();
    match matches.subcommand_name() {
        Some("install") => {
            let insert_command: dyn Command = InstallCommand::new(&execute_helm_commands);
            execute_helm_commands.run(&insert_command);
        }
        //        Some("upgrade") => handle_install(&matches, &matches.subcommand_name(), helm_home_dir),
        Some("upgrade") => {
            let upgrade_command: dyn Command = UpgradeCommand::new(&execute_helm_commands);
            execute_helm_commands.run(&upgrade_command);
        }
        _ => {}
    }

    match matches.subcommand_name() {
        Some("upgrade") => handle_upgrade(&matches, &matches.subcommand_name(), helm_home_dir),
        Some("install") => handle_install(&matches, &matches.subcommand_name(), helm_home_dir),
        _ => {}
    }
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
