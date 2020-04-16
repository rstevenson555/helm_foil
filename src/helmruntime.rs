use std::collections::HashMap;
use std::fs;

use clap::ArgMatches;
use regex::Regex;

use std::borrow::Borrow;
use std::fmt::Display;
use std::fs::File;
use std::hash::Hash;
use std::io::Write;
use std::path::Path;
use std::process::{Command as ProcessCommand, Output};

#[derive(Debug, Clone)]
pub(crate) struct HelmRuntime {
    implicit_variables: HashMap<String, String>,
    explicit_variables: HashMap<String, String>,
}

impl HelmRuntime {
    pub(crate) fn new() -> HelmRuntime {
        HelmRuntime {
            implicit_variables: HashMap::new(),
            explicit_variables: HashMap::new(),
        }
    }

    pub(crate) fn set_implicit_var(&mut self, key: String, value: String) {
        self.implicit_variables.insert(key, value);
    }

    pub(crate) fn set_explicit_var(&mut self, key: String, value: String) {
        self.explicit_variables.insert(key, value);
    }

    /**
    The idea here is that you can use any type as a lookup key, as long as that type could be “borrowed” from the stored key type.
    http://idubrov.name/rust/2018/06/01/tricking-the-hashmap.html
    **/
    fn get_implicit_var<K: ?Sized>(&self, key: &K) -> Option<&String>
    where
        String: Borrow<K>,
        K: Hash + Eq,
    {
        self.implicit_variables.get(key)
    }

    /**
    The idea here is that you can use any type as a lookup key, as long as that type could be “borrowed” from the stored key type.
    http://idubrov.name/rust/2018/06/01/tricking-the-hashmap.html
    **/
    fn get_explicit_var<K: ?Sized>(&self, key: &K) -> Option<&String>
    where
        String: Borrow<K>,
        K: Hash + Eq,
    {
        self.explicit_variables.get(key)
    }

    fn read_values_file(&self, filename: &str) -> String {
        let error_msg = format!("Something went wrong reading the file {}", filename);
        let estr: &str = error_msg.as_str();

        fs::read_to_string(filename).expect(estr)
    }

    /*
    make the pattern that we are matching against
    */
    fn make_regex_pattern<K: ?Sized>(&self, var: &K) -> String
    where
        String: Borrow<K>,
        K: Display,
    {
        format!("(?i)\\{{\\{{\\s*{}\\s*\\}}\\}}", var)
    }

    fn replace_implicit_vars(&self, result: &mut String) {
        if let Ok(release_name_pattern) =
            Regex::new(self.make_regex_pattern(".Release.Name").as_str())
        {
            if let Some(var) = self.get_implicit_var("release.name") {
                *result = release_name_pattern
                    .replace_all(result.as_str(), var.as_str())
                    .into_owned();
            }
        }
        if let Ok(chart_name_pattern) = Regex::new(self.make_regex_pattern(".Chart.Name").as_str())
        {
            if let Some(var) = self.get_implicit_var("chart.name") {
                *result = chart_name_pattern
                    .replace_all(result.as_str(), var.as_str())
                    .into_owned();
            }
        }

        if let Ok(branch_name_pattern) =
            Regex::new(self.make_regex_pattern(".Branch.Name").as_str())
        {
            if let Some(var) = self.get_implicit_var("source.branch") {
                *result = branch_name_pattern
                    .replace_all(result.as_str(), var.as_str())
                    .into_owned();
            }
        }

        if let Ok(previous_branch_pattern) =
            Regex::new(self.make_regex_pattern(".Previous.Branch").as_str())
        {
            if let Some(var) = self.get_implicit_var("previous.branch") {
                *result = previous_branch_pattern
                    .replace_all(result.as_str(), var.as_str())
                    .into_owned();
            }
        }

        if let Ok(starting_canary_percentage_pattern) = Regex::new(
            self.make_regex_pattern(".Starting.Canary.Percentage")
                .as_str(),
        ) {
            if let Some(var) = self.get_implicit_var("starting.canary.percentage") {
                *result = starting_canary_percentage_pattern
                    .replace_all(result.as_str(), var.as_str())
                    .into_owned();
            }
        }
    }

    fn replace_explicit_vars(&self, override_file_result: &mut String, pattern_str: &str) {
        if let Ok(pattern) = Regex::new(pattern_str) {
            if let Some(var) = self.get_explicit_var(pattern_str) {
                *override_file_result = pattern
                    .replace_all(override_file_result.as_str(), var.as_str())
                    .into_owned();
            }
        }
    }

    pub(crate) fn get_and_set_chart_name(
        &mut self,
        upgrade_command: &ArgMatches,
        helm_command: &mut ProcessCommand,
    ) {
        if let Some(chart_path) = upgrade_command.value_of("CHART") {
            helm_command.arg(chart_path);
            let path = Path::new(chart_path);
            // add global variable key/value 'chart.name'
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    self.set_implicit_var("chart.name".to_string(), filename_str.to_string());
                }
                // add global variable key/value 'chart.path'
                self.set_implicit_var("chart.path".to_string(), chart_path.to_string());
            }
        }
    }

    pub(crate) fn apply_common_args(
        &mut self,
        global_args: &ArgMatches,
        subcommand: &ArgMatches,
        helm_command: &mut ProcessCommand,
    ) {
        let config_env_yaml: &mut String = &mut "".to_string();
        let values_yaml: &mut String = &mut "".to_string();

        match self.get_implicit_var("chart.path") {
            Some(chart_path) => {
                *values_yaml =
                    self.read_values_file(format!("{}/values.yaml", chart_path).as_str());
                // VALUES file
                self.replace_implicit_vars(values_yaml);
            }
            None => {
                panic!("[helm] missing chart specified on the command line");
            }
        }

        let override_filename: &mut String = &mut "".to_string();
        if let Some(override_file) = subcommand.value_of("valueFiles") {
            *override_filename = override_file.to_string();
            helm_command.args(&["-f", override_filename]);
            *config_env_yaml = self.read_values_file(override_filename);
        }

        let set_values_collection: Vec<&str>;
        if let Some(set_values) = subcommand.values_of("set") {
            set_values_collection = set_values.collect();
            // loop over all --sets on the command line
            for set_var in set_values_collection.iter() {
                let split_parts: Vec<&str> = set_var.split('=').collect();

                // convert the --set arguments on the command line to global variables formatted like
                // {{.SetKey}}; example image.tag becomes {{.Values.image.tag}} template variable
                let variable_format =
                    self.make_regex_pattern(format!(".Values.{}", split_parts[0]).as_str());
                println!("set template variable {}", variable_format);

                self.set_explicit_var(variable_format.to_owned(), split_parts[1].to_owned());
                self.set_implicit_var(split_parts[0].to_owned(), split_parts[1].to_owned());

                // now replace the explicit variables declared from the command line from the -f override file
                // env CONFIG/*.yaml files
                self.replace_explicit_vars(config_env_yaml, &variable_format);

                // now replace the explicit variables declared from the command line from the chart/values.yaml file
                // VALUES file
                self.replace_explicit_vars(values_yaml, &variable_format);

                helm_command.args(&["--set", (*set_var).to_string().as_str()]);
            }

            // replace values.yaml contents
            self.replace_implicit_vars(values_yaml);

            if subcommand.is_present("valueFiles") {
                // replace global vars
                // create more implicit variables in config/*.yaml
                self.replace_implicit_vars(config_env_yaml);
            }

            println!("{}", config_env_yaml); // => "xxxxx xxxxx!"
        }

        if let Some(tiller_namespace) = global_args.value_of("tiller-namespace") {
            helm_command.args(&["--tiller-namespace", tiller_namespace]);
        }
        if let Some(namespace) = global_args.value_of("namespace") {
            helm_command.args(&["--namespace", namespace]);
        }
        if let Some(timeout) = global_args.value_of("timeout") {
            helm_command.args(&["--timeout", timeout]);
        }
        if global_args.is_present("debug") {
            helm_command.arg("--debug");
        }

        // write output
        HelmRuntime::write_env_override_file(config_env_yaml, override_filename);

        self.write_values_file(values_yaml)
    }

    fn write_values_file(&mut self, values_yaml: &mut String) {
        if let Some(chart_path) = self.get_implicit_var("chart.path") {
            match File::create(format!("{}/values.yaml", chart_path)) {
                Ok(mut file) => {
                    if let Err(err) = file.write_all(values_yaml.as_bytes()) {
                        panic!("[helm] Error writing file {}", err);
                    }
                }
                Err(e) => {
                    panic!("[helm] Error writing values file out {}", e);
                }
            }
        }
    }

    fn write_env_override_file(config_env_yaml: &mut String, override_filename: &mut String) {
        match File::create(override_filename) {
            Ok(mut file) => {
                if let Err(err) = file.write_all(config_env_yaml.as_bytes()) {
                    panic!("[helm] Error writing file {}", err);
                }
            }
            Err(e) => {
                panic!("[helm] Error writing override file {}", e);
            }
        }
    }

    pub(crate) fn execute_helm(&self, helm_command: &mut ProcessCommand) {
        println!("about to execute {:?}", helm_command);
        let output: Output = helm_command
            .spawn()
            .expect("[helm] failed to spawn helm")
            .wait_with_output()
            .expect("[helm] failed to wait on helm to complete");

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
}
