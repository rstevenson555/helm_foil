use clap::ArgMatches;

use crate::executehelm::HelmRuntime;

pub(crate) trait Command<'a> {
    // Traits can provide default method definitions.
    fn get_helm_runtime(&self) -> &HelmRuntime;

    fn run(&self, matches: &ArgMatches, command: &Option<&str>, helm_home_dir: String);
}
