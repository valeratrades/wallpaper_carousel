#![feature(duration_constructors)]
use std::{
	path::{Path, PathBuf},
	process::Command as ProcessCommand,
};

use clap::Parser;
use color_eyre::{
	Result,
	eyre::{Context, ContextCompat, bail, ensure},
};
use rand::prelude::IndexedRandom;
use serde::Deserialize;
use tracing::{info, warn};
use v_utils::utils::eyre::exit_on_error;
use wallpaper_carousel::config::{AppConfig, SettingsFlags};

const BILLIONAIRE_CACHE_TTL: std::time::Duration = std::time::Duration::from_weeks(1);
const BLURB_CACHE_TTL: std::time::Duration = std::time::Duration::from_days(1);
#[derive(Debug, Parser)]
#[command(name = "wallpaper_carousel")]
#[command(about = "Extend wallpaper with citation overlays")]
struct Args {
	#[command(subcommand)]
	command: Command,
	#[command(flatten)]
	settings: SettingsFlags,
}
#[derive(Debug, Parser)]
enum Command {
	/// Extend an image with text overlays and set as wallpaper
	Extend {
		/// Path to input image file (jpg or png). If not provided, uses the last input file from cache.
		input: Option<PathBuf>,
	},

	/// Generate wallpaper using the bundled vision document
	Generate,

	/// Circle through images in the same directory
	Circle {
		/// Go forwards
		#[arg(short, long, conflicts_with_all = ["backwards", "random"])]
		forward: bool,

		/// Go backwards
		#[arg(short, long, conflicts_with_all = ["forward", "random"])]
		backwards: bool,

		/// Select a random image
		#[arg(short, long, conflicts_with_all = ["forward", "backwards"])]
		random: bool,

		/// Optional directory to use instead of the parent of last input
		directory: Option<PathBuf>,
	},
}
fn main() {
	v_utils::clientside!();
	exit_on_error(run());
}

#[derive(Debug, Deserialize)]
struct SwayOutput {
	/// None for inactive outputs (e.g., unplugged HDMI)
	current_mode: Option<CurrentMode>,
}

#[derive(Debug, Deserialize)]
struct CurrentMode {
	width: u32,
	height: u32,
}

#[derive(Clone, Debug)]
struct SafeArea {
	x: u32,
	y: u32,
	width: u32,
	height: u32,
}

#[derive(Debug, Deserialize)]
struct ForbesRtb {
	#[serde(rename = "personList")]
	person_list: ForbesPersonList,
}

#[derive(Debug, Deserialize)]
struct ForbesPersonList {
	#[serde(rename = "personsLists")]
	persons_lists: Vec<Person>,
}

#[derive(Debug, Deserialize)]
struct Person {
	#[serde(rename = "personName")]
	person_name: String,
	/// millions of USD
	#[serde(rename = "finalWorth")]
	final_worth: f64,
	#[serde(default)]
	country: Option<String>,
	#[serde(default)]
	source: Option<String>,
	#[serde(default)]
	industries: Vec<String>,
	#[serde(default)]
	age: Option<u32>,
	#[serde(rename = "selfMade", default)]
	self_made: Option<bool>,
	#[serde(default)]
	bios: Vec<String>,
}

/// The annual World's Billionaires List. The real-time list (`person/rtb/0`) undercounts it - it only carries fortunes Forbes can price intraday.
fn billionaire_list() -> Result<Vec<Person>> {
	let cache_path = v_utils::xdg_cache_file!("billionaires.json");
	if cache_path.exists() {
		let age = cache_path.metadata()?.modified()?.elapsed()?;
		if age < BILLIONAIRE_CACHE_TTL {
			return parse_forbes(&std::fs::read(&cache_path)?);
		}
	}

	let year: u32 = String::from_utf8(ProcessCommand::new("date").arg("+%Y").output().wrap_err("Failed to run date")?.stdout)?
		.trim()
		.parse()?;
	resolve_list(year, |y| {
		let raw = fetch_forbes(y)?;
		let list = parse_forbes(&raw)?;
		if !list.is_empty() {
			std::fs::write(&cache_path, &raw)?;
		}
		Ok(list)
	})
}

fn resolve_list(year: u32, fetch: impl Fn(u32) -> Result<Vec<Person>>) -> Result<Vec<Person>> {
	let list = match fetch(year)? {
		l if l.is_empty() => fetch(year - 1)?, // new list drops in April, until then the current year's URL is served empty
		l => l,
	};
	ensure!(!list.is_empty(), "Forbes has no list for either {year} or {}", year - 1);
	Ok(list)
}

fn fetch_forbes(year: u32) -> Result<Vec<u8>> {
	let output = ProcessCommand::new("curl")
		.args([
			"-sS",
			"--max-time",
			"15",
			"-A",
			"Mozilla/5.0",
			&format!(
				"https://www.forbes.com/forbesapi/person/billionaires/{year}/position/true.json?fields=rank,personName,finalWorth,country,source,industries,age,selfMade,bios&limit=4000"
			),
		])
		.output()
		.wrap_err("Failed to run curl")?;

	if !output.status.success() {
		bail!("curl failed: {}", String::from_utf8_lossy(&output.stderr));
	}
	Ok(output.stdout)
}

fn parse_forbes(raw: &[u8]) -> Result<Vec<Person>> {
	let rtb: ForbesRtb = serde_json::from_slice(raw).wrap_err("Forbes list schema changed")?;
	Ok(rtb.person_list.persons_lists)
}

/// Spotlight on one billionaire, cached for the day so `circle` doesn't pay for a call per wallpaper switch.
fn billionaire_blurb(list: &[Person], reroll: bool) -> Result<String> {
	let cache_path = v_utils::xdg_cache_file!("billionaire_blurb.txt");
	if !reroll && cache_path.exists() {
		let age = cache_path.metadata()?.modified()?.elapsed()?;
		if age < BLURB_CACHE_TTL {
			return Ok(std::fs::read_to_string(&cache_path)?);
		}
	}
	// `ask_llm` panics on a missing key while building the client, which would cost us the wallpaper
	ensure!(std::env::var_os("CLAUDE_TOKEN").is_some(), "CLAUDE_TOKEN not set");

	let p = list.choose(&mut rand::rng()).context("Empty billionaire list")?;
	let prompt = format!(
		"{name}. Net worth ${worth:.1}B. Country: {country}. Source: {source}. Industries: {industries}. Age: {age}. Self-made: {self_made}.\nForbes bios:\n{bios}\n\n\
		Write ~40 words on how this person built their fortune and what the business actually does. Only the money: drop hobbies, family, philanthropy, awards, residences, politics. \
		Plain prose, no markdown, no preamble, lead with the name.",
		name = p.person_name,
		worth = p.final_worth / 1000.,
		country = p.country.as_deref().unwrap_or("unknown"),
		source = p.source.as_deref().unwrap_or("unknown"),
		industries = p.industries.join(", "),
		age = p.age.map(|a| a.to_string()).unwrap_or("unknown".to_owned()),
		self_made = p.self_made.map(|s| s.to_string()).unwrap_or("unknown".to_owned()),
		bios = p.bios.join("\n"),
	);

	let response = tokio::runtime::Runtime::new()?.block_on(ask_llm::Client::default().model(ask_llm::Model::Fast).max_tokens(200).ask(prompt))?;
	let blurb = response.text.trim().to_owned();
	std::fs::write(&cache_path, &blurb)?;
	Ok(blurb)
}

fn billionaire_stats(list: &[Person], blurb: Option<String>) -> Vec<String> {
	let mut stats = vec![format!("{} billionaires", list.len())];
	stats.extend(blurb);
	stats
}

fn get_cache_file_path() -> PathBuf {
	v_utils::xdg_cache_file!("last_input.txt")
}

fn get_lock_file_path() -> PathBuf {
	v_utils::xdg_state_file!("wallpaper_generation.lock")
}

fn get_supported_image_extensions() -> Vec<&'static str> {
	// what typst can embed
	vec!["jpg", "jpeg", "png", "gif", "webp", "svg"]
}

/// `--root /` so `photo.typ` can read a background from anywhere on disk. `output{n}.png` lets us
/// detect multi-page documents, which mean the overlay no longer fits and we refuse to render.
fn compile_wallpaper(doc: &Path, overlay: &serde_json::Value, ppi: f32) -> Result<PathBuf> {
	ensure!(doc.exists(), "Typst document does not exist: {}", doc.display());
	let output_path = v_utils::xdg_state_file!("extended.png");
	let stem = output_path.with_extension("");
	let page = |n: u8| PathBuf::from(format!("{}{n}.png", stem.display()));
	for n in 1..=2 {
		if page(n).exists() {
			std::fs::remove_file(page(n))?;
		}
	}

	let output = ProcessCommand::new("typst")
		.args(["compile", "--format", "png", "--root", "/", "--ppi"])
		.arg(ppi.to_string())
		.arg("--input")
		.arg(format!("overlay={overlay}"))
		.arg(doc)
		.arg(format!("{}{{n}}.png", stem.display()))
		.output()
		.wrap_err("Failed to run typst")?;
	ensure!(output.status.success(), "typst compilation failed:\n{}", String::from_utf8_lossy(&output.stderr));
	ensure!(!page(2).exists(), "{} rendered more than 1 page: the overlay no longer fits", doc.display());

	std::fs::rename(page(1), &output_path)?;
	Ok(output_path)
}

fn find_next_image(current_path: &Path, backwards: bool, directory: Option<&Path>) -> Result<PathBuf> {
	let parent = if let Some(dir) = directory {
		dir
	} else {
		current_path.parent().context("Current image has no parent directory")?
	};

	// Get all image files in the directory
	let mut image_files: Vec<PathBuf> = std::fs::read_dir(parent)?
		.filter_map(|entry| entry.ok())
		.map(|entry| entry.path())
		.filter(|path| {
			path.is_file()
				&& path
					.extension()
					.and_then(|ext| ext.to_str())
					.map(|ext| get_supported_image_extensions().contains(&ext.to_lowercase().as_str()))
					.unwrap_or(false)
		})
		.collect();

	if image_files.is_empty() {
		bail!("No images found in directory: {}", parent.display());
	}

	// Sort files for consistent ordering
	image_files.sort();

	if image_files.len() == 1 {
		bail!("Only one image in directory: {}", parent.display());
	}

	// Find current file index - if directory was provided and current file is not in it,
	// start from the first or last image depending on direction
	let current_index = image_files.iter().position(|p| p == current_path);

	// Calculate next index
	let next_index = match current_index {
		Some(idx) =>
			if backwards {
				if idx == 0 { image_files.len() - 1 } else { idx - 1 }
			} else {
				(idx + 1) % image_files.len()
			},
		None => {
			// Current file not in this directory, start from beginning or end
			if backwards { image_files.len() - 1 } else { 0 }
		}
	};

	Ok(image_files[next_index].clone())
}

fn find_random_image(current_path: &Path, directory: Option<&Path>) -> Result<PathBuf> {
	let parent = if let Some(dir) = directory {
		dir
	} else {
		current_path.parent().context("Current image has no parent directory")?
	};

	// Get all image files in the directory
	let mut image_files: Vec<PathBuf> = std::fs::read_dir(parent)?
		.filter_map(|entry| entry.ok())
		.map(|entry| entry.path())
		.filter(|path| {
			path.is_file()
				&& path
					.extension()
					.and_then(|ext| ext.to_str())
					.map(|ext| get_supported_image_extensions().contains(&ext.to_lowercase().as_str()))
					.unwrap_or(false)
		})
		.collect();

	if image_files.is_empty() {
		bail!("No images found in directory: {}", parent.display());
	}

	// Sort files for consistent ordering
	image_files.sort();

	// Remove current file from the list (only if it's in this directory)
	image_files.retain(|p| p != current_path);

	if image_files.is_empty() {
		bail!("Only one image in directory: {}", parent.display());
	}

	// Select a random image
	let random_image = image_files.choose(&mut rand::rng()).context("Failed to select random image")?;

	Ok(random_image.clone())
}

fn check_and_handle_lock() -> Result<()> {
	let lock_path = get_lock_file_path();

	if lock_path.exists() {
		// Read PID from lock file
		let pid_str = std::fs::read_to_string(&lock_path)?;
		let pid_str = pid_str.trim();
		if pid_str.is_empty() {
			// Stale/corrupt lock file with no PID — just remove it
			std::fs::remove_file(&lock_path)?;
			return Ok(());
		}
		let pid: i32 = pid_str.parse().context("Invalid PID in lock file")?;

		// Try to kill the process
		v_utils::elog!("Found existing process (PID: {}), killing it...", pid);
		// SAFETY: We're sending SIGTERM to a process we know exists (read from lock file).
		// The PID is validated to be a valid i32. SIGTERM is a safe signal to send.
		unsafe {
			libc::kill(pid, libc::SIGTERM);
		}

		// Wait a bit for the process to terminate
		std::thread::sleep(std::time::Duration::from_millis(100));

		// Remove the lock file
		std::fs::remove_file(&lock_path)?;
	}

	Ok(())
}

fn create_lock() -> Result<()> {
	let lock_path = get_lock_file_path();
	if let Some(parent) = lock_path.parent() {
		std::fs::create_dir_all(parent)?;
	}

	let pid = std::process::id();
	std::fs::write(&lock_path, pid.to_string())?;

	Ok(())
}

fn remove_lock() -> Result<()> {
	let lock_path = get_lock_file_path();
	if lock_path.exists() {
		std::fs::remove_file(&lock_path)?;
	}
	Ok(())
}

fn save_last_input(path: &Path) -> Result<()> {
	let cache_path = get_cache_file_path();
	if let Some(parent) = cache_path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(&cache_path, path.to_string_lossy().as_bytes())?;
	Ok(())
}

fn load_last_input() -> Result<PathBuf> {
	let cache_path = get_cache_file_path();
	let content = std::fs::read_to_string(&cache_path).context(
		"No input file provided and no cached input file found.\n\
		Please provide an input file: wallpaper_carousel <path-to-image>",
	)?;
	Ok(PathBuf::from(content.trim()))
}

fn generate_wallpaper(input_path: &Path, config: &AppConfig, reroll: bool) -> Result<()> {
	info!("Starting wallpaper generation for: {}", input_path.display());

	// Select a random quote
	let quote = config.quotes.choose(&mut rand::rng()).context("No quotes configured")?;
	v_utils::elog!("Selected quote: {:?}", quote.text);
	v_utils::elog!("Author: {:?}", quote.author);

	let mut stats: Vec<String> = Vec::new();

	if let Some(balance) = &config.balance {
		match balance.get_value() {
			Ok(value) => match &balance.label {
				Some(label) => {
					v_utils::elog!("{}:\n{}", label, value);
					stats.push(format!("{label}\n{value}"));
				}
				None => {
					v_utils::elog!("{}", value);
					stats.push(value);
				}
			},
			Err(e) => warn!("Balance command failed: {e}"),
		}
	}

	if config.billionaires {
		// Forbes' endpoint intermittently stalls behind bot protection; a missing decoration must not cost us the wallpaper.
		match billionaire_list() {
			Ok(list) => {
				let blurb = billionaire_blurb(&list, reroll).inspect_err(|e| warn!("Billionaire blurb failed: {e}")).ok(); // the count is worth rendering on its own
				for line in billionaire_stats(&list, blurb) {
					v_utils::elog!("{line}");
					stats.push(line);
				}
			}
			Err(e) => warn!("Billionaire list failed: {e}"),
		}
	}

	let (display_width, display_height) = get_display_resolution()?;
	let all_displays = get_all_active_displays()?;
	v_utils::elog!("Found {} active display(s)", all_displays.len());
	for (i, (w, h)) in all_displays.iter().enumerate() {
		v_utils::elog!("  Display {}: {}x{} (ratio: {:.3})", i + 1, w, h, *w as f32 / *h as f32);
	}

	// The page is always PAGE_WIDTH wide; `--ppi` alone scales the raster to the display, so text keeps its relative size.
	const PAGE_WIDTH: u32 = 1920;
	let to_pt = |px: u32| (px as f32 * PAGE_WIDTH as f32 / display_width as f32).round() as u32;
	let padding = config.text_padding.unwrap_or(15);

	let safe_area = calculate_safe_area(display_width, display_height, &all_displays);
	v_utils::elog!(
		"Safe area: x={}, y={}, width={}, height={} ({:.1}% of the frame)",
		safe_area.x,
		safe_area.y,
		safe_area.width,
		safe_area.height,
		(safe_area.width * safe_area.height) as f32 / (display_width * display_height) as f32 * 100.0
	);

	let mut overlay = serde_json::json!({
		"quote": quote.text,
		"author": quote.author,
		"stats": stats,
		"width": PAGE_WIDTH,
		"height": to_pt(display_height),
		"inset": {
			"top": to_pt(safe_area.y) + padding,
			"bottom": to_pt(display_height - safe_area.y - safe_area.height) + padding,
			"left": to_pt(safe_area.x) + padding,
			"right": to_pt(display_width - safe_area.x - safe_area.width) + padding,
		},
	});

	// A `.typ` input is the vision document itself; anything else is a photo we hand to `photo.typ` as a background.
	let doc = match input_path.extension().is_some_and(|e| e == "typ") {
		true => input_path.to_owned(),
		false => {
			overlay["bg"] = serde_json::json!(std::fs::canonicalize(input_path)?); // typst resolves it against `--root /`
			config.vision_source.as_ref().parent().context("Vision source has no parent directory")?.join("photo.typ")
		}
	};

	let output_path = compile_wallpaper(&doc, &overlay, 72. * display_width as f32 / PAGE_WIDTH as f32)?;

	ProcessCommand::new("swaymsg")
		.args(["output", "*", "background", output_path.to_str().unwrap(), "fill"])
		.output()?;

	v_utils::log!("Wallpaper set to {}", output_path.display());

	Ok(())
}

fn handle_next_command(backwards: bool, random: bool, directory: Option<PathBuf>) -> Result<()> {
	info!("Circle command: backwards={backwards}, random={random}, directory={directory:?}");

	// Load the current image path
	let current_path = load_last_input()?;

	// Determine which directory to use
	let target_dir = if let Some(ref dir) = directory {
		dir.as_path()
	} else {
		current_path.parent().context("Current image has no parent directory")?
	};
	v_utils::log!("Directory: {}", target_dir.display());

	// Find next image
	let next_path = if random {
		find_random_image(&current_path, directory.as_deref())?
	} else {
		find_next_image(&current_path, backwards, directory.as_deref())?
	};
	v_utils::log!("Next image: {}", next_path.display());

	// Check for existing lock and kill if necessary
	check_and_handle_lock()?;

	// Set wallpaper immediately with the original next image (sway handles resizing)
	ProcessCommand::new("swaymsg").args(["output", "*", "background", next_path.to_str().unwrap(), "fill"]).output()?;
	v_utils::log!("Wallpaper set to: {}", next_path.display());

	// Save the next path to cache
	save_last_input(&next_path)?;

	// Spawn a separate background process to generate text overlay
	// We use std::process::Command instead of thread::spawn because when the main
	// process exits, spawned threads are killed. A separate process continues independently.
	let current_exe = std::env::current_exe()?;
	ProcessCommand::new(current_exe)
		.arg("extend")
		.arg(&next_path)
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()?;

	v_utils::log!("Text overlay generation started in background...");

	Ok(())
}

fn run() -> Result<()> {
	let args = Args::parse();

	// Handle subcommands
	match args.command {
		Command::Circle {
			forward,
			backwards,
			random,
			directory,
		} => {
			// Require at least one flag
			if !forward && !backwards && !random {
				bail!("Please specify either --forward, --backwards, or --random");
			}
			// backwards takes precedence if both are somehow set, then random
			handle_next_command(backwards, random, directory)
		}
		Command::Extend { input } => {
			// Load config from CLI flags
			let config = AppConfig::try_build(args.settings)?;

			// Check and handle existing lock (kill previous background process if running)
			check_and_handle_lock()?;

			// Create lock for this process
			create_lock()?;

			// Determine input path: use provided arg or load from cache
			let input_path = match input {
				Some(path) => path,
				None => load_last_input()?,
			};

			// Generate wallpaper
			let result = generate_wallpaper(&input_path, &config, false);

			// Remove lock
			remove_lock()?;

			// Save the input path to cache for next time
			save_last_input(&input_path)?;

			result
		}
		Command::Generate => {
			// Load config from CLI flags
			let config = AppConfig::try_build(args.settings)?;

			// Check and handle existing lock (kill previous background process if running)
			check_and_handle_lock()?;

			// Create lock for this process
			create_lock()?;

			let vision_path = config.vision_source.to_path_buf();
			v_utils::log!("Using vision document: {}", vision_path.display());

			let result = generate_wallpaper(&vision_path, &config, true);

			// Remove lock
			remove_lock()?;

			// Save the vision path to cache (so extend without args also uses vision)
			save_last_input(&vision_path)?;

			result
		}
	}
}

fn get_display_resolution() -> Result<(u32, u32)> {
	// Find the smallest (most square) display to target
	// This way on wider monitors we'll have unfilled space instead of cropping
	let all_displays = get_all_active_displays()?;
	if all_displays.is_empty() {
		bail!("No active outputs found");
	}

	// Find the display with the smallest area (width * height)
	// This tends to be the more "square" one since ultra-wide monitors have larger areas
	let (width, height) = all_displays.iter().min_by_key(|(w, h)| w * h).copied().context("No active outputs found")?;

	Ok((width, height))
}

fn get_all_active_displays() -> Result<Vec<(u32, u32)>> {
	let output = ProcessCommand::new("swaymsg").args(["-t", "get_outputs"]).output()?;
	if !output.status.success() {
		bail!("swaymsg -t get_outputs failed:\n{}", String::from_utf8_lossy(&output.stderr));
	}
	let outputs: Vec<SwayOutput> = serde_json::from_slice(&output.stdout)?;
	Ok(outputs.iter().filter_map(|o| o.current_mode.as_ref().map(|m| (m.width, m.height))).collect())
}

fn calculate_safe_area(img_width: u32, img_height: u32, displays: &[(u32, u32)]) -> SafeArea {
	// For each display, calculate how the image would be cropped when using "fill" mode
	// "fill" scales the image to cover the entire screen, then crops the excess

	let img_ratio = img_width as f32 / img_height as f32;

	let mut min_x = 0;
	let mut min_y = 0;
	let mut max_x = img_width;
	let mut max_y = img_height;

	for &(display_width, display_height) in displays {
		let display_ratio = display_width as f32 / display_height as f32;

		// Calculate how the image would be scaled and cropped for this display
		let (scaled_width, _scaled_height, x_offset, y_offset) = if img_ratio > display_ratio {
			// Image is wider than display - will crop horizontally
			let scaled_height = display_height;
			let scaled_width = (display_height as f32 * img_ratio) as u32;
			let x_offset = (scaled_width - display_width) / 2;
			(scaled_width, scaled_height, x_offset, 0)
		} else {
			// Image is taller than display - will crop vertically
			let scaled_width = display_width;
			let scaled_height = (display_width as f32 / img_ratio) as u32;
			let y_offset = (scaled_height - display_height) / 2;
			(scaled_width, scaled_height, 0, y_offset)
		};

		// Convert the cropped area back to original image coordinates
		let scale_factor = img_width as f32 / scaled_width as f32;
		let crop_x_start = (x_offset as f32 * scale_factor) as u32;
		let crop_y_start = (y_offset as f32 * scale_factor) as u32;
		let crop_x_end = crop_x_start + (display_width as f32 * scale_factor) as u32;
		let crop_y_end = crop_y_start + (display_height as f32 * scale_factor) as u32;

		// Update the safe area to be the intersection of all cropped areas
		min_x = min_x.max(crop_x_start);
		min_y = min_y.max(crop_y_start);
		max_x = max_x.min(crop_x_end);
		max_y = max_y.min(crop_y_end);
	}

	SafeArea {
		x: min_x,
		y: min_y,
		width: max_x.saturating_sub(min_x),
		height: max_y.saturating_sub(min_y),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const FIXTURE: &str = r#"{"personList":{"personsLists":[
		{"rank":1,"finalWorth":342000.0,"personName":"Elon Musk","age":53,"country":"United States","source":"Tesla, SpaceX","industries":["Automotive"],"selfMade":true,"bios":["Elon Musk cofounded seven companies."]},
		{"rank":2900,"finalWorth":1000.0,"personName":"Nameless Tail","country":"China","source":"lithium","industries":["Manufacturing"],"selfMade":true}
	],"count":2}}"#;

	#[test]
	fn parses_forbes_payloads() {
		let list = parse_forbes(FIXTURE.as_bytes()).unwrap();
		assert_eq!(list.len(), 2);
		assert_eq!(list[0].person_name, "Elon Musk");
		assert_eq!(list[0].final_worth, 342000.0);
		assert_eq!(list[1].bios.len(), 0);
		assert_eq!(list[1].age, None);
		assert_eq!(parse_forbes(br#"{"personList":{"personsLists":[],"count":0}}"#).unwrap().len(), 0);
	}

	#[test]
	fn empty_current_year_falls_back() {
		let fetch = |y| if y == 2026 { Ok(Vec::new()) } else { parse_forbes(FIXTURE.as_bytes()) };
		assert_eq!(resolve_list(2026, fetch).unwrap().len(), 2);
		assert!(resolve_list(2026, |_| Ok(Vec::new())).is_err());
	}

	#[test]
	fn count_survives_a_missing_blurb() {
		let list = parse_forbes(FIXTURE.as_bytes()).unwrap();
		assert_eq!(billionaire_stats(&list, None), vec!["2 billionaires"]);
		assert_eq!(billionaire_stats(&list, Some("a ".repeat(40).trim().to_owned()))[0], "2 billionaires");
	}
}
