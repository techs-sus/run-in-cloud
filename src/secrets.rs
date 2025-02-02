use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

const SECRET_FILE: &str = "key.txt";

#[derive(Serialize, Deserialize)]
pub struct Secrets {
	pub key: String,
	pub universe_id: u64,
	pub place_id: u64,
}

fn get_directory() -> Result<PathBuf, anyhow::Error> {
	let directory = directories::ProjectDirs::from("com", "techs-sus", "run-in-cloud")
		.context("failed getting project directories")?
		.data_local_dir()
		.to_owned();

	std::fs::create_dir_all(&directory)?;
	Ok(directory)
}

pub fn read_secrets() -> Result<Secrets, anyhow::Error> {
	let directory = get_directory()?;
	let contents = std::fs::read(directory.join(SECRET_FILE))?;

	Ok(serde_json::from_slice(&contents)?)
}

pub fn write_secrets(secrets: &Secrets) -> Result<(), anyhow::Error> {
	std::fs::write(
		get_directory()?.join(SECRET_FILE),
		&serde_json::to_string(secrets)?,
	)?;

	Ok(())
}
