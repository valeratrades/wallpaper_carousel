use std::process::Command;

use color_eyre::eyre::{Result, WrapErr as _, bail};
use serde::{Deserialize, Deserializer};
use v_utils::{io::ExpandedPath, macros::MyConfigPrimitives};

#[derive(Clone, Debug, Default, MyConfigPrimitives)]
pub struct AppConfig {
	pub quotes: Vec<Quote>,
}

#[derive(Clone, Debug)]
pub struct Quote {
	pub text: String,
	pub author: Option<String>,
}

impl<'de> Deserialize<'de> for Quote {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>, {
		#[derive(Deserialize)]
		#[serde(untagged)]
		enum QuoteHelper {
			String(String),
			Structured { text: String, author: Option<String> },
		}

		let helper = QuoteHelper::deserialize(deserializer)?;
		Ok(match helper {
			QuoteHelper::String(text) => Quote { text, author: None },
			QuoteHelper::Structured { text, author } => Quote { text, author },
		})
	}
}

impl AppConfig {
	pub fn read(path: Option<ExpandedPath>) -> Result<Self> {
		let app_name = env!("CARGO_PKG_NAME");
		let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name);
		let xdg_conf_dir = xdg_dirs.get_config_home().expect("config home").parent().unwrap().display().to_string();

		let locations = [
			format!("{xdg_conf_dir}/{app_name}"),
			format!("{xdg_conf_dir}/{app_name}/config"), //
		];

		let mut builder = config::Config::builder().add_source(config::Environment::default());

		match path {
			Some(path) => {
				let path_str = path.to_string();
				if path_str.ends_with(".nix") {
					// Evaluate .nix file to JSON
					let json_str = Self::eval_nix_file(&path_str)?;
					let builder = builder.add_source(config::File::from_str(&json_str, config::FileFormat::Json));
					Ok(builder.build()?.try_deserialize()?)
				} else {
					let builder = builder.add_source(config::File::with_name(&path_str).required(true));
					Ok(builder.build()?.try_deserialize()?)
				}
			}
			None => {
				// Check for .nix file first, then fall back to other formats
				let nix_path = format!("{xdg_conf_dir}/{app_name}.nix");
				if std::path::Path::new(&nix_path).exists() {
					let json_str = Self::eval_nix_file(&nix_path)?;
					let builder = builder.add_source(config::File::from_str(&json_str, config::FileFormat::Json));
					return Ok(builder.build()?.try_deserialize()?);
				}

				// Fall back to TOML and other formats
				for location in locations.iter() {
					builder = builder.add_source(config::File::with_name(location).required(false));
				}
				let raw: config::Config = builder.build()?;

				raw.try_deserialize().wrap_err("Config file does not exist or is invalid")
			}
		}
	}

	fn eval_nix_file(path: &str) -> Result<String> {
		let output = Command::new("nix")
			.arg("eval")
			.arg("--json")
			.arg("--impure")
			.arg("--expr")
			.arg(format!("import {}", path))
			.output()
			.wrap_err("Failed to execute nix command. Is nix installed?")?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			bail!("Nix evaluation failed: {}", stderr);
		}

		Ok(String::from_utf8(output.stdout)?)
	}
}
