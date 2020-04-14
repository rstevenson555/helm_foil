use clap::ArgMatches;

use crate::helmruntime::HelmRuntime;

pub(crate) trait Command<'a> {
    // Traits can provide default method definitions.
    fn get_helm_runtime(&mut self) -> &mut HelmRuntime;

    fn run(&mut self, matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String);

    fn echo(&self, string: &str) {
        println!("{}", string);
    }
}
