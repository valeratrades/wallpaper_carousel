use std::process::Command;

use color_eyre::eyre::{Result, WrapErr as _, bail};
use serde::{Deserialize, Deserializer};
use v_utils::{
	macros::{MyConfigPrimitives, Settings},
	prelude::ExpandedPath,
};

#[derive(Clone, Debug, MyConfigPrimitives, Settings)]
pub struct AppConfig {
	pub quotes: Vec<Quote>,
	pub balance: Option<Balance>,
	/// Render today's count of billionaires worldwide, fetched from Forbes' real-time list.
	#[serde(default = "yes")]
	pub billionaires: bool,
	pub text_padding: Option<u32>,
	/// Path to the typst (`.typ`) document compiled into the generated wallpaper.
	pub vision_source: ExpandedPath,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Balance {
	pub command: String,
	pub label: Option<String>,
}
impl Balance {
	pub fn get_value(&self) -> Result<String> {
		let output = Command::new("sh").arg("-c").arg(&self.command).output().wrap_err("Failed to execute balance command")?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			bail!("Balance command failed: {stderr}");
		}

		let stdout = String::from_utf8(output.stdout)?;
		Ok(stdout.trim().to_string())
	}
}

#[derive(Clone, Debug)]
pub struct Quote {
	pub text: String,
	pub author: Option<String>,
}
fn yes() -> bool {
	true
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
