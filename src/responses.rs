use std::collections::{hash_map::Entry, HashMap};
use std::fmt;
use std::path::PathBuf;

use base64::encode;
use jsonpath_lib as jsonpath;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{from_reader, to_string, Value};

#[derive(Debug)]
enum Error {
	ResponseQuery(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::ResponseQuery(s) => write!(f, "{}", s),
		}
	}
}

lazy_static! {
	static ref RE: Regex = Regex::new("\\{response:[^}]*\\}").unwrap();
}

pub struct Responses {
	path: PathBuf,
	contents: HashMap<String, Value>,
	file_reader: fn(PathBuf) -> Result<Box<dyn std::io::Read>, Box<dyn std::error::Error>>,
}

impl Responses {
	fn read_file(path: PathBuf) -> Result<Box<dyn std::io::Read>, Box<dyn std::error::Error>> {
		Ok(Box::new(std::fs::File::open(path)?))
	}

	pub fn new(path: PathBuf) -> Self {
		Self {
			path,
			contents: HashMap::new(),
			file_reader: Responses::read_file,
		}
	}

	// get is a helper functions that attempts to get the contents of
	// a response file. If we haven't received it yet, we try to open
	// the file and read it as JSON.
	fn get(&mut self, response: &str) -> Result<&Value, Box<dyn std::error::Error>> {
		Ok(match self.contents.entry(response.to_string()) {
			Entry::Occupied(json) => json.into_mut(),
			Entry::Vacant(v) => {
				let file = self.path.join(encode(&response));
				let file = (self.file_reader)(file)?;
				v.insert(from_reader(file)?)
			}
		})
	}

	/// Apply any response queries to the given string. As a side
	/// effect, responses that have not yet been read are pulled into
	/// memory so they can be queried. If the response queries
	/// jsonpath would produce more than one result, only the first
	/// result is returned. That is not to say an array wouldn't be
	/// returned but rather that if the query has multiple hits, the
	/// first hit is returned.
	pub fn apply(&mut self, s: &str) -> Result<String, Box<dyn std::error::Error>> {
		let mut s = s.to_string();

		let rs = s.clone();
		let rqs = ResponseQuery::from(&rs)?;
		for query in rqs {
			let value = match jsonpath::select(self.get(query.response)?, query.query) {
				Ok(value) => match value.len() {
					0 => {
						return Err(Box::new(Error::ResponseQuery(format!(
							"json path not found: {}",
							query.query
						))))
					}
					_ => value[0],
				},
				Err(err) => return Err(Box::new(err)),
			};
			s = s.replace(query.original, &to_string(value)?)
		}

		Ok(s)
	}
}

#[derive(Debug, PartialEq)]
pub struct ResponseQuery<'a> {
	pub original: &'a str,
	pub response: &'a str,
	pub query: &'a str,
}

impl<'a> ResponseQuery<'a> {
	// Search the string for all the response queries that need to be
	// replaced. A response query has for form
	// ```{response:[name]:[jsonpath]}```.  The name should be the name
	// of the request the response is associated with and the jsonpath
	// should be a jsonpath (.e.g $.child.id).
	pub fn from(s: &str) -> Result<Vec<ResponseQuery>, Box<dyn std::error::Error>> {
		let mut queries = vec![];

		for find in RE.find_iter(s) {
			let original = find.as_str();
			let parts = original
				.clone()
				.trim_start_matches("{response:")
				.trim_end_matches("}");
			let parts: Vec<&str> = parts.splitn(2, ":").collect();
			if parts.len() != 2 {
				return Err(Box::new(Error::ResponseQuery(format!(
					"invalid response query: {}",
					original
				))));
			}
			queries.push(ResponseQuery {
				original,
				response: parts[0],
				query: parts[1],
			});
		}

		Ok(queries)
	}
}

#[cfg(test)]
mod tests {
	use crate::responses::*;

	#[test]
	fn test_response_query_from() {
		assert_eq!(
			ResponseQuery::from("{response:foo:bar}").unwrap(),
			vec![ResponseQuery {
				original: "{response:foo:bar}",
				response: "foo",
				query: "bar"
			}]
		);

		assert_eq!(
			ResponseQuery::from("{response:foo:bar} {response:bar:baz}").unwrap(),
			vec![
				ResponseQuery {
					original: "{response:foo:bar}",
					response: "foo",
					query: "bar"
				},
				ResponseQuery {
					original: "{response:bar:baz}",
					response: "bar",
					query: "baz"
				}
			]
		);

		assert_eq!(ResponseQuery::from("").unwrap(), vec![]);

		assert!(
			ResponseQuery::from("{response:foo.bar}").is_err(),
			"bad response didn't error"
		);
	}

	#[test]
	fn test_responses_apply() {}
}
