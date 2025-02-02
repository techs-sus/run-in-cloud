mod api;
mod secrets;

use clap::{Parser, Subcommand};
use std::{io::Write, path::PathBuf, time::Duration};

const CHECKMARK_SYMBOL: &str = "\u{2714}";

#[derive(Parser)]
struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	/// Login with an Open Cloud API key.
	Login {
		/// A key to be used in Open Cloud.
		#[arg(short, long)]
		key: String,

		/// The target place id to be used in Open Cloud.
		#[arg(short, long)]
		universe_id: u64,

		/// The target universe id to be used in Open Cloud.
		#[arg(short, long)]
		place_id: u64,
	},

	/// Runs a task, waits for it to finish, and prints it's execution logs.
	Run {
		/// A path to the place file to use in Open Cloud.
		#[arg(long)]
		place: PathBuf,

		/// A path to the script file to run in Open Cloud.
		#[arg(long)]
		script: PathBuf,
	},
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
	let args = Args::parse();

	match args.command {
		Command::Login {
			key,
			universe_id,
			place_id,
		} => secrets::write_secrets(&secrets::Secrets {
			key,
			universe_id,
			place_id,
		})?,

		Command::Run { place, script } => {
			let secrets = secrets::read_secrets()?;
			let client = api::get_roblox_request_client(&secrets.key)?;
			let target_version = {
				let mut publish_spinner = spinners::Spinner::new(
					spinners::Spinners::Arrow,
					"waiting for task to finish executing...".to_owned(),
				);

				let version =
					api::publish_place(&client, secrets.universe_id, secrets.place_id, &place).await?;
				publish_spinner.stop_and_persist(
					CHECKMARK_SYMBOL,
					"successfully published to place!".to_owned(),
				);
				version
			};

			let path = api::start_luau_execution_task(
				&client,
				target_version,
				secrets.universe_id,
				secrets.place_id,
				std::fs::read_to_string(script)?,
			)
			.await?;

			let task_endpoint = format!("https://apis.roblox.com/cloud/v2/{path}");

			// ignore this, it's just a terminal spinner
			let mut task_spinner = spinners::Spinner::new(
				spinners::Spinners::Arrow,
				"waiting for task to finish executing...".to_owned(),
			);

			let mut time: u64 = 3;
			let mut retries: u16 = 3;

			// poll the execution task; uses exponential backoff
			let finished_task_response = loop {
				match api::get_task_response(&client, &task_endpoint).await {
					Err(..) => {
						retries -= 1;
						time *= 2;
						eprintln!("failed fetching task info, {retries} left");

						if retries == 0 {
							eprintln!("no retries left, exiting");
							std::process::exit(1);
						}
					}
					Ok(task_response) => {
						if task_response.state == "COMPLETE" || task_response.state == "FAILED" {
							break task_response;
						}
					}
				}

				tokio::time::sleep(Duration::from_secs(time)).await;
			};

			task_spinner.stop_and_persist(CHECKMARK_SYMBOL, "task finished executing!".to_owned());

			let mut logs: Vec<api::Log> = vec![];
			let mut next_page_token = String::new();

			loop {
				let response: api::Logs = serde_json::from_str(
					&client
						.get(format!(
							"{task_endpoint}/logs?maxPageSize=10000&pageToken={next_page_token}"
						))
						.send()
						.await?
						.text()
						.await?,
				)?;

				logs.extend(response.logs);

				if response.next_page_token.is_empty() {
					break;
				};

				next_page_token = response.next_page_token;
			}

			let mut lock = std::io::stdout();
			for message in logs.into_iter().flat_map(|log| log.messages) {
				lock.write_all(message.as_bytes())?;
				lock.write_all(b"\n")?;
			}

			lock.flush()?;

			if let Some(error) = finished_task_response.error {
				eprintln!("got error with code {}: {}", error.code, error.message);
			}
		}
	}

	Ok(())
}
