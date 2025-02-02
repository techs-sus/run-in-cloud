mod api;
mod secrets;

use api::TaskState;
use clap::{Parser, Subcommand};
use owo_colors::{OwoColorize, Stream};
use spinners::{Spinner, Spinners};
use std::{io::Write, path::PathBuf, time::Duration};

const CHECKMARK_SYMBOL: &str = "\u{2714}";

#[derive(Parser)]
struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	/// Login with an Open Cloud API key and a target experience.
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
			let client = api::create_authenticated_client(&secrets.key)?;

			// we need the place version so that if another task is queued with a different place, our task is not affected
			// [task 1 -> publish azalea.rbxl @ version 32]
			// [task 1 -> start @ version 32]
			// [... other tasks]
			// [task 4 -> publish tests.rbxl @ version 35]
			// [task 4 -> start @ version 35]
			let place_version = {
				let mut publish_spinner = Spinner::new(
					Spinners::Arrow,
					"waiting for place to get published..."
						.if_supports_color(Stream::Stdout, |text| text.blue())
						.to_string(),
				);

				let version =
					api::publish_place(&client, secrets.universe_id, secrets.place_id, &place).await?;

				publish_spinner.stop_and_persist(
					CHECKMARK_SYMBOL,
					"successfully published to place!"
						.if_supports_color(Stream::Stdout, |text| text.green())
						.to_string(),
				);
				version
			};

			let path = api::create_luau_execution_task(
				&client,
				place_version,
				secrets.universe_id,
				secrets.place_id,
				std::fs::read_to_string(script)?,
			)
			.await?;

			let task_endpoint = format!("https://apis.roblox.com/cloud/v2/{path}");

			let mut task_spinner = Spinner::new(
				Spinners::Arrow,
				"waiting for task to finish executing..."
					.if_supports_color(Stream::Stdout, |text| text.blue())
					.to_string(),
			);

			let mut time: u64 = 3;
			let mut retries: u16 = 3;

			// poll the execution task; uses exponential backoff
			let finished_task_response = loop {
				match api::get_task_response(&client, &task_endpoint).await {
					Err(..) => {
						retries -= 1;
						time *= 2;
						eprintln!(
							"{}",
							"failed fetching task info, {retries} left"
								.if_supports_color(Stream::Stdout, |text| text.red())
						);

						if retries == 0 {
							eprintln!(
								"{}",
								"no retries left, exiting".if_supports_color(Stream::Stdout, |text| text.red())
							);
							std::process::exit(1);
						}
					}
					Ok(task_response) => {
						if task_response.state == TaskState::Complete
							|| task_response.state == TaskState::Failed
						{
							break task_response;
						}
					}
				}

				tokio::time::sleep(Duration::from_secs(time)).await;
			};

			let task_finished_text = match (
				finished_task_response.create_time,
				finished_task_response.update_time,
			) {
				(Some(create_time), Some(update_time)) => {
					let duration = humantime::parse_rfc3339(&update_time)?
						.duration_since(humantime::parse_rfc3339(&create_time)?)?;

					&format!(
						"task finished executing in {}!",
						humantime::format_duration(duration)
					)
				}
				_ => "task finished executing!",
			};

			task_spinner.stop_and_persist(
				CHECKMARK_SYMBOL,
				task_finished_text
					.if_supports_color(Stream::Stdout, |text| text.green())
					.to_string(),
			);

			let logs = api::get_all_logs(&client, &task_endpoint).await?;

			let mut stdout_lock = std::io::stdout().lock();
			for message in logs.into_iter().flat_map(|log| log.messages) {
				stdout_lock.write_all(message.as_bytes())?;
				stdout_lock.write_all(b"\n")?;
			}
			stdout_lock.flush()?;

			if let Some(error) = finished_task_response.error {
				eprintln!(
					"{} {}{} {}",
					"task errored with code".if_supports_color(Stream::Stdout, |text| text.red()),
					error
						.code
						.if_supports_color(Stream::Stdout, |text| text.bright_red()),
					":".if_supports_color(Stream::Stdout, |text| text.red()),
					error
						.message
						.if_supports_color(Stream::Stdout, |text| text.bright_red()),
				);
			}
		}
	}

	Ok(())
}
