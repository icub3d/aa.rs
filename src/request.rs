use std::collections::HashMap;
use std::{fmt, path::PathBuf, str, str::FromStr};

use hyper::body::HttpBody as _;
use hyper::Client;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{stdout, AsyncWriteExt as _};
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::environment::{ApplyEnvironment, Environment};
use crate::responses::Responses;

fn default_scheme() -> String {
    "http".to_string()
}

fn default_path() -> String {
    "/".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Body {
    File { path: String, content_type: String },
    String { body: String, content_type: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    method: String,

    #[serde(default = "default_scheme")]
    scheme: String,

    host: String,

    #[serde(default = "default_path")]
    path: String,

    #[serde(default)]
    headers: HashMap<String, String>,

    #[serde(default)]
    query: HashMap<String, String>,

    body: Option<Body>,
}

impl Request {
    pub fn apply_responses(&mut self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut responses = Responses::new(path);
        self.scheme = responses.apply(&self.scheme)?;
        self.host = responses.apply(&self.host)?;
        self.path = responses.apply(&self.path)?;
        self.headers = self
            .headers
            .iter()
            .map(|(hk, hv)| Ok((responses.apply(hk)?, responses.apply(&hv)?)))
            .collect::<Result<HashMap<String, String>, Box<dyn std::error::Error>>>()?;
        self.query = self
            .query
            .iter()
            .map(|(qk, qv)| Ok((responses.apply(qk)?, responses.apply(qv)?)))
            .collect::<Result<HashMap<String, String>, Box<dyn std::error::Error>>>()?;
        Ok(())
    }

    pub fn apply_environment(&mut self, envs: &[Environment]) {
        self.scheme = envs.apply_environment(&self.scheme);
        self.host = envs.apply_environment(&self.host);
        self.path = envs.apply_environment(&self.path);
        self.headers = self
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), envs.apply_environment(v)))
            .collect();
        self.query = self
            .query
            .iter()
            .map(|(k, v)| (k.clone(), envs.apply_environment(v)))
            .collect();
    }

    pub async fn run(
        &mut self,
        envs: &[Environment],
        save: PathBuf,
        responses: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.apply_environment(envs);
        self.apply_responses(responses)?;

        let client: Client<_, hyper::Body> =
            Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots());

        let method = hyper::Method::from_str(&self.method.to_ascii_uppercase())?;

        let query = self
            .query
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<String>();

        let pq = match query.len() > 0 {
            true => format!("{}?{}", self.path, query),
            false => format!("{}", self.path),
        };

        let uri = hyper::Uri::builder()
            .authority(self.host.as_str())
            .scheme(self.scheme.as_str())
            .path_and_query(pq)
            .build()
            .unwrap();

        let body = match &self.body {
            None => hyper::Body::empty(),
            Some(body) => match body {
                Body::String { body, content_type } => {
                    self.headers
                        .insert("Content-Type".to_string(), content_type.clone());
                    hyper::Body::from(body.clone())
                }
                Body::File { path, content_type } => {
                    self.headers
                        .insert("Content-Type".to_string(), content_type.clone());

                    let file = File::open(path).await?;
                    let stream = FramedRead::new(file, BytesCodec::new());
                    hyper::Body::wrap_stream(stream)
                }
            },
        };

        let mut req = hyper::Request::builder();
        for (k, v) in self.headers.iter() {
            req = req.header(k, v);
        }
        let req = req.method(method).uri(uri).body(body).unwrap();

        let mut resp = client.request(req).await?;
        println!("{}", resp.status());

        let mut file = File::create(save).await?;

        while let Some(chunk) = resp.body_mut().data().await {
            let chunk = chunk?;
            // TODO select?
            file.write_all(&chunk).await?;
            stdout().write_all(&chunk).await?;
        }
        Ok(())
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "method: {}\n", self.method)?;
        write!(f, "uri: {}://{}{}\n", self.scheme, self.host, self.path)?;
        if self.headers.len() > 0 {
            write!(f, "headers:\n")?;
        }
        for (key, value) in &self.headers {
            write!(f, "  {}: {}\n", key, value)?;
        }
        if self.query.len() > 0 {
            write!(f, "query-parameters:\n")?;
        }
        for (key, value) in &self.query {
            write!(f, "  {}: {}\n", key, value)?;
        }
        Ok(())
    }
}
