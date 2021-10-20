use std::collections::HashMap;
use std::path::Path;

use base64::encode;
use serde::{Deserialize, Serialize};

use crate::{environment::Environment, request::Request};

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
	#[serde(default)]
	path: String,

	#[serde(default)]
	environments: HashMap<String, Environment>,
	#[serde(default)]
	requests: HashMap<String, Request>,
}

impl Project {
	pub fn new(path: String) -> Self {
		Project {
			path,
			environments: HashMap::new(),
			requests: HashMap::new(),
		}
	}

	pub fn print_environments(&self, envs: Vec<String>, verbose: bool) {
		for (name, env) in &self.environments {
			if !envs.contains(name) && envs.len() > 0 {
				continue;
			}
			println!("{}", name);
			if verbose {
				for (k, v) in env {
					println!("  {}={}", k, v);
				}
			}
		}
	}

	pub fn print_requests(&mut self, envs: Vec<String>, requests: Vec<String>, verbose: bool) {
		for (name, req) in &mut self.requests {
			if !requests.contains(name) && requests.len() > 0 {
				continue;
			}

			if verbose {
				let envs: Vec<Environment> = self
					.environments
					.iter()
					.filter(|(k, _)| envs.contains(k))
					.map(|(_, v)| v.to_owned())
					.collect();
				req.apply_environment(&envs);
				println!("name: {}", name);
				println!("{}", req);
				println!("");
			} else {
				println!("{}", name);
			}
		}
	}

	pub async fn run_request(
		&mut self,
		envs: &[String],
		req: String,
	) -> Result<(), Box<dyn std::error::Error>> {
		let envs: Vec<Environment> = self
			.environments
			.iter()
			.filter(|(k, _)| envs.contains(k))
			.map(|(_, v)| v.to_owned())
			.collect();
		if let Some(r) = self.requests.get_mut(&req) {
			let responses = Path::new(&self.path).join("responses");
			std::fs::create_dir_all(&responses)?;
			let path = responses.join(encode(&req));
			r.run(&envs, path, responses).await?;
		}
		Ok(())
	}

	pub fn extend(&mut self, p: Self) {
		self.environments.extend(p.environments);
		self.requests.extend(p.requests);
	}
}
