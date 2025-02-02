use anyhow::Context;
use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Saves a place and returns the version number.
pub async fn publish_place(
	client: &Client,
	universe_id: u64,
	place_id: u64,
	place: &PathBuf,
) -> Result<u64, anyhow::Error> {
	#[derive(Deserialize)]
	struct Response {
		#[serde(rename = "versionNumber")]
		version_number: u64,
	}

	let content_type = match place
		.extension()
		.context("no file extension")?
		.to_str()
		.context("failed converting extension to string")?
	{
		"rbxl" => "application/octet-stream",
		"rbxlx" => "application/xml",

		_ => anyhow::bail!("file extension not supported"),
	};

	let version: Response = serde_json::from_str(
		&client
			.post(format!(
				"https://apis.roblox.com/universes/v1/{universe_id}/places/{place_id}/versions?VersionType=Published"
			))
			.header("content-type", content_type)
			.body(std::fs::read(place)?)
			.send()
			.await?
			.text()
			.await?,
	)?;

	Ok(version.version_number)
}

#[derive(Deserialize)]
pub struct Error {
	pub code: String,
	pub message: String,
}

#[derive(Deserialize)]
pub struct Output {
	pub results: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct TaskResponse {
	pub path: String,
	// user: String,
	pub state: String,
	// script: String,
	pub error: Option<Error>,
	pub output: Option<Output>,
}

/// Starts an execution task, only returning the task id.
pub async fn start_luau_execution_task(
	client: &Client,
	place_version: u64,
	universe_id: u64,
	place_id: u64,
	script: String,
) -> Result<String, anyhow::Error> {
	#[derive(Serialize)]
	struct Body {
		script: String,
	}

	let body = serde_json::to_string(&Body { script })?;

	let response: TaskResponse = serde_json::from_str(&client.post(
		format!("https://apis.roblox.com/cloud/v2/universes/{universe_id}/places/{place_id}/versions/{place_version}/luau-execution-session-tasks")
	).body(body).header("Content-Type", "application/json").send().await?.text().await?)?;

	Ok(response.path)
}

pub async fn get_task_response(client: &Client, url: &str) -> Result<TaskResponse, anyhow::Error> {
	Ok(serde_json::from_str(
		&client.get(url).send().await?.text().await?,
	)?)
}

#[derive(Deserialize, Debug)]
pub struct Log {
	// path: String,
	pub messages: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Logs {
	#[serde(rename = "luauExecutionSessionTaskLogs")]
	pub logs: Vec<Log>,
	#[serde(rename = "nextPageToken")]
	pub next_page_token: String,
}

pub fn get_roblox_request_client(key: &str) -> Result<Client, anyhow::Error> {
	const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

	let mut headers = HeaderMap::new();
	headers.insert("x-api-key", HeaderValue::from_str(key)?);

	Ok(
		Client::builder()
			.https_only(true)
			.default_headers(headers)
			.user_agent(USER_AGENT)
			.build()?,
	)
}
