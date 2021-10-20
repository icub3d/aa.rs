use std::fs::File;

use clap::{AppSettings, Clap};
use serde_yaml::from_reader;
use walkdir::WalkDir;

use aa::project::Project;

/// aa - API Automation tool in the terminal.
#[derive(Clap)]
#[clap(version = "1.0", author = "Joshua Marsh <joshua@themarshians.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Options {
	/// Path to the directory of the aa project you want to use.
	#[clap(short, long, env = "AA_PROJECT_PATH", default_value = "~/.config/aa")]
	project_path: String,

	/// A comma separated list of environments to load. They are
	/// loaded in the order given.
	#[clap(short, long, value_delimiter = ',')]
	environments: Option<String>,

	#[clap(subcommand)]
	command: Commands,
}

#[derive(Clap)]
enum Commands {
	/// List the environments in the project.
	#[clap(name = "env")]
	Environments(EnvironmentsCommand),

	/// Performs operations with requests in the project.
	#[clap(name = "req", subcommand)]
	Requests(RequestsCommands),
}

#[derive(Clap)]
struct EnvironmentsCommand {
	/// Print out the values in the environment as well.
	#[clap(short)]
	verbose: bool,

	/// If any positional arguments are given, only print them out.
	#[clap()]
	environments: Vec<String>,
}

#[derive(Clap)]
enum RequestsCommands {
	/// List the requests in the project.
	#[clap()]
	List(RequestsList),

	/// Run the given requests.
	#[clap()]
	Run(RequestsRun),
}

#[derive(Clap)]
struct RequestsList {
	/// Print out the details of the request as well.
	#[clap(short)]
	verbose: bool,

	/// If any positional arguments are given, only print them out.
	#[clap()]
	requests: Vec<String>,
}

#[derive(Clap)]
struct RequestsRun {
	/// The list of requests to run.
	#[clap()]
	requests: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let options = Options::parse();

	let envs: Vec<String> = match options.environments {
		Some(ee) => ee.split(",").map(|s| s.to_string()).collect(),
		None => Vec::new(),
	};

	let mut project = parse_environment(&options.project_path)?;

	match options.command {
		Commands::Environments(e) => project.print_environments(e.environments, e.verbose),
		Commands::Requests(r) => match r {
			RequestsCommands::List(l) => project.print_requests(envs, l.requests, l.verbose),
			RequestsCommands::Run(r) => {
				for req in r.requests {
					project.run_request(&envs, req).await?;
				}
			}
		},
	};

	Ok(())
}

fn parse_environment(path: &str) -> Result<Project, Box<dyn std::error::Error>> {
	// Walk through the project path files and load all the yaml files
	// into a single project.
	let mut project = Project::new(path.to_string());
	for entry in WalkDir::new(path).follow_links(true) {
		let entry = entry?;
		let yaml = match entry.path().extension() {
			Some(ext) => ext == "yaml",
			None => false,
		};
		if !entry.file_type().is_dir() && yaml {
			let file = File::open(entry.path())?;
			let np: Project = from_reader(file)?;
			project.extend(np);
		}
	}
	Ok(project)
}
