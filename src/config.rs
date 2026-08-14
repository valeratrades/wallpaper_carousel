use std::process::Command;

use color_eyre::eyre::{Result, WrapErr as _, bail};
use serde::{Deserialize, Deserializer, Serialize};
use v_utils::{
	io::ExpandedPath,
	macros::{MyConfigPrimitives, Settings},
};

#[derive(Clone, Debug, MyConfigPrimitives, Settings)]
pub struct AppConfig {
	pub quotes: Vec<Quote>,
	pub balance: Option<Balance>,
	/// Render the count of billionaires worldwide, plus a paragraph on how a random one of them made their money.
	#[serde(default = "yes")]
	pub billionaires: bool,
	/// Breathing room between the text and the edge of the safe area, in pt of a 1920pt-wide page.
	pub edge_padding: Option<u32>,
	/// Path to the typst (`.typ`) document compiled into the generated wallpaper. `overlay.typ` and `photo.typ` are looked up next to it.
	pub vision_source: ExpandedPath,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
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
