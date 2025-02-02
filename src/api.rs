use anyhow::Context;
use reqwest::{
	header::{HeaderMap, HeaderValue},
	Client,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Publishes a place with the given `client`, `universe_id`, `place_id`, and `place`.
/// Returns the newly published version number.
pub async fn publish_place(
	client: &Client,
	universe_id: u64,
	place_id: u64,
	place: &PathBuf,
) -> Result<u64, anyhow::Error> {
	#[derive(Deserialize)]
	struct PublishPlaceResponse {
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

	let publish_response: PublishPlaceResponse =
		client
			.post(format!(
				"https://apis.roblox.com/universes/v1/{universe_id}/places/{place_id}/versions?VersionType=Published"
			))
			.header("content-type", content_type)
			.body(std::fs::read(place)?)
			.send()
			.await?
			.json()
			.await?;

	Ok(publish_response.version_number)
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

#[derive(Deserialize, PartialEq, Eq)]
pub enum TaskState {
	#[serde(rename = "STATE_UNSPECIFIED")]
	Unspecified,
	#[serde(rename = "QUEUED")]
	Queued,
	#[serde(rename = "PROCESSING")]
	Processing,
	#[serde(rename = "CANCELLED")]
	Cancelled,
	#[serde(rename = "COMPLETE")]
	Complete,
	#[serde(rename = "FAILED")]
	Failed,
}

#[derive(Deserialize)]
pub struct TaskResponse {
	pub path: String,
	// user: String, // why? you don't need this
	pub state: TaskState,
	#[serde(rename = "createTime")]
	pub create_time: Option<String>,
	#[serde(rename = "updateTime")]
	pub update_time: Option<String>,
	// script: String, // redundant because we already have src
	pub error: Option<Error>,
	pub output: Option<Output>,
}

/// Creates a execution task, only returning the task path.
pub async fn create_luau_execution_task(
	client: &Client,
	place_version: u64,
	universe_id: u64,
	place_id: u64,
	script: String,
) -> Result<String, anyhow::Error> {
	#[derive(Serialize)]
	struct TaskCreateBody {
		script: String,
	}

	let response: TaskResponse = client.post(
		format!("https://apis.roblox.com/cloud/v2/universes/{universe_id}/places/{place_id}/versions/{place_version}/luau-execution-session-tasks")
	).json(&TaskCreateBody { script }).send().await?.json().await?;

	Ok(response.path)
}

/// Gets a task response. You must ensure that the url is a valid task response endpoint.
pub async fn get_task_response(client: &Client, url: &str) -> Result<TaskResponse, anyhow::Error> {
	Ok(client.get(url).send().await?.json().await?)
}

/// Gets all logs for the `task_endpoint` specified.
pub async fn get_all_logs(client: &Client, task_endpoint: &str) -> Result<Vec<Log>, anyhow::Error> {
	let mut logs: Vec<Log> = vec![];
	let mut next_page_token = String::new();

	loop {
		let response: Logs = client
			.get(format!(
				"{task_endpoint}/logs?maxPageSize=10000&pageToken={next_page_token}"
			))
			.send()
			.await?
			.json()
			.await?;

		logs.extend(response.logs);

		if response.next_page_token.is_empty() {
			break;
		};

		next_page_token = response.next_page_token;
	}

	Ok(logs)
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

pub fn create_authenticated_client(key: &str) -> Result<Client, anyhow::Error> {
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
