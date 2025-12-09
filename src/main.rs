use std::{
	path::{Path, PathBuf},
	process::Command as ProcessCommand,
};

use clap::Parser;
use color_eyre::{
	Result,
	eyre::{Context, ContextCompat, bail},
};
use image::GenericImageView;
use rand::prelude::IndexedRandom;
use serde::Deserialize;
use tracing::info;
use v_utils::utils::eyre::exit_on_error;
use wallpaper_carousel::config::{AppConfig, SettingsFlags};

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

#[derive(Debug, Deserialize)]
struct SwayOutput {
	current_mode: CurrentMode,
	active: bool,
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

struct CompositeParams<'a> {
	bg_image_path: &'a Path,
	output_path: &'a Path,
	text: &'a str,
	author: Option<&'a str>,
	balance: Option<&'a str>,
	width: u32,
	height: u32,
	safe_area: &'a SafeArea,
	text_padding: u32,
}

fn get_cache_file_path() -> PathBuf {
	v_utils::xdg_cache_file!("last_input.txt")
}

fn get_lock_file_path() -> PathBuf {
	v_utils::xdg_state_file!("wallpaper_generation.lock")
}

fn get_supported_image_extensions() -> Vec<&'static str> {
	// Based on image crate's supported formats
	vec!["jpg", "jpeg", "png", "gif", "webp", "bmp", "ico", "tiff", "tif"]
}

fn get_vision_paths() -> Result<(PathBuf, PathBuf)> {
	// Returns (vision.png path, src_typ directory path)
	// When installed via nix, structure is: $out/bin/wallpaper_carousel and $out/share/vision/
	let exe_path = std::env::current_exe()?;
	let exe_dir = exe_path.parent().context("Executable has no parent directory")?;

	// Go up from bin/ to $out/, then into share/vision/
	let vision_dir = exe_dir.parent().context("bin directory has no parent")?.join("share/vision");
	let vision_png = vision_dir.join("vision.png");
	let src_typ = vision_dir.join("src_typ");

	if vision_png.exists() && src_typ.exists() {
		return Ok((vision_png, src_typ));
	}

	// Fallback: check if we're in development
	let cwd = std::env::current_dir()?;
	let dev_png = cwd.join("src_typ/output.png");
	let dev_src = cwd.join("src_typ");

	if dev_src.exists() {
		return Ok((dev_png, dev_src));
	}

	bail!(
		"Could not find bundled vision files. Checked:\n  - {}\n  - {}\nMake sure the package was built with `nix build`.",
		vision_dir.display(),
		dev_src.display()
	)
}

fn get_newest_source_mtime(src_typ_dir: &Path) -> Result<std::time::SystemTime> {
	let mut newest = std::time::SystemTime::UNIX_EPOCH;

	for entry in walkdir::WalkDir::new(src_typ_dir).into_iter().filter_map(|e| e.ok()) {
		if entry.file_type().is_file() {
			let path = entry.path();
			// Skip the output files
			if path.file_name().map(|n| n.to_string_lossy().starts_with("output")).unwrap_or(false) {
				continue;
			}
			if let Ok(metadata) = path.metadata() {
				if let Ok(mtime) = metadata.modified() {
					if mtime > newest {
						newest = mtime;
					}
				}
			}
		}
	}

	Ok(newest)
}

fn regenerate_vision_if_needed() -> Result<PathBuf> {
	let (vision_png, src_typ) = get_vision_paths()?;

	// Check if we need to regenerate
	let needs_regeneration = if vision_png.exists() {
		let png_mtime = vision_png.metadata()?.modified()?;
		let src_mtime = get_newest_source_mtime(&src_typ)?;
		src_mtime > png_mtime
	} else {
		true
	};

	if needs_regeneration {
		v_utils::log!("Vision sources are newer than output, regenerating...");

		// Create a temporary directory for compilation
		let temp_dir = std::env::temp_dir().join("wallpaper_carousel_typst");
		std::fs::create_dir_all(&temp_dir)?;

		// Copy source files to temp dir (in case src_typ is read-only in nix store)
		for entry in walkdir::WalkDir::new(&src_typ).into_iter().filter_map(|e| e.ok()) {
			let rel_path = entry.path().strip_prefix(&src_typ)?;
			let dest = temp_dir.join(rel_path);
			if entry.file_type().is_dir() {
				std::fs::create_dir_all(&dest)?;
			} else if entry.file_type().is_file() {
				if let Some(parent) = dest.parent() {
					std::fs::create_dir_all(parent)?;
				}
				std::fs::copy(entry.path(), &dest)?;
			}
		}

		// Compile with typst
		let output = ProcessCommand::new("typst")
			.args(["compile", "--format", "png", "vision.typ", "output{n}.png"])
			.current_dir(&temp_dir)
			.output()?;

		if !output.status.success() {
			bail!("typst compilation failed:\n{}", String::from_utf8_lossy(&output.stderr));
		}

		// Check for single page
		if temp_dir.join("output2.png").exists() {
			bail!("Error: More than 1 page generated. Vision document must be single-page.");
		}

		// Copy output to vision_png location (if writable) or to a cache location
		let output_png = temp_dir.join("output1.png");
		let final_path = if std::fs::copy(&output_png, &vision_png).is_ok() {
			vision_png
		} else {
			// Can't write to nix store, use a cache location
			let cache_vision = v_utils::xdg_cache_file!("vision.png");
			std::fs::copy(&output_png, &cache_vision)?;
			cache_vision
		};

		v_utils::log!("Regenerated vision document: {}", final_path.display());
		Ok(final_path)
	} else {
		Ok(vision_png)
	}
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
		let pid: i32 = pid_str.trim().parse().context("Invalid PID in lock file")?;

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

fn main() {
	v_utils::clientside!();
	exit_on_error(run());
}

fn generate_wallpaper(input_path: &Path, config: &AppConfig) -> Result<()> {
	info!("Starting wallpaper generation for: {}", input_path.display());

	// Select a random quote
	let quote = config.quotes.choose(&mut rand::rng()).context("No quotes configured")?;
	v_utils::elog!("Selected quote: {:?}", quote.text);
	v_utils::elog!("Author: {:?}", quote.author);

	// Get balance value if configured
	let balance_text = if let Some(balance) = &config.balance {
		let value = balance.get_value()?;
		if let Some(label) = &balance.label {
			v_utils::elog!("{}:\n{}", label, value);
			Some(format!("{}\n{}", label, value))
		} else {
			v_utils::elog!("{}", value);
			Some(value)
		}
	} else {
		None
	};

	v_utils::log!("Generating CSS...");

	// Get display resolution from swaymsg
	let (display_width, display_height) = get_display_resolution()?;

	// Get all active displays to calculate safe area
	let all_displays = get_all_active_displays()?;
	v_utils::elog!("Found {} active display(s)", all_displays.len());
	for (i, (w, h)) in all_displays.iter().enumerate() {
		v_utils::elog!("  Display {}: {}x{} (ratio: {:.3})", i + 1, w, h, *w as f32 / *h as f32);
	}

	// Save resized background image to temp location
	let temp_bg_path = v_utils::xdg_state_file!("background_temp.png");
	let img = image::open(input_path)?;
	let resized_img = resize_fill(img, display_width, display_height);
	let (img_width, img_height) = resized_img.dimensions();
	resized_img.save(&temp_bg_path)?;

	// Calculate safe area that will be visible on all monitors
	let safe_area = calculate_safe_area(img_width, img_height, &all_displays);
	v_utils::elog!(
		"Safe area: x={}, y={}, width={}, height={} ({:.1}% of image)",
		safe_area.x,
		safe_area.y,
		safe_area.width,
		safe_area.height,
		(safe_area.width * safe_area.height) as f32 / (img_width * img_height) as f32 * 100.0
	);

	// Composite text onto background image
	let text_padding = config.text_padding.unwrap_or(15);
	let output_path = v_utils::xdg_state_file!("extended.png");
	composite_text_on_image(&CompositeParams {
		bg_image_path: &temp_bg_path,
		output_path: &output_path,
		text: &quote.text,
		author: quote.author.as_deref(),
		balance: balance_text.as_deref(),
		width: img_width,
		height: img_height,
		safe_area: &safe_area,
		text_padding,
	})?;

	// Set wallpaper using swaymsg
	ProcessCommand::new("swaymsg")
		.args(["output", "*", "background", output_path.to_str().unwrap(), "fill"])
		.output()?;

	v_utils::log!("Wallpaper set to {}", output_path.display());

	Ok(())
}

fn handle_next_command(backwards: bool, random: bool, directory: Option<PathBuf>) -> Result<()> {
	info!("Circle command: backwards={}, random={}, directory={:?}", backwards, random, directory);

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
			let result = generate_wallpaper(&input_path, &config);

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

			// Get the bundled vision image path, regenerating if needed
			let vision_path = regenerate_vision_if_needed()?;
			v_utils::log!("Using vision image: {}", vision_path.display());

			// Generate wallpaper using the vision document
			let result = generate_wallpaper(&vision_path, &config);

			// Remove lock
			remove_lock()?;

			// Save the vision path to cache (so extend without args also uses vision)
			save_last_input(&vision_path)?;

			result
		}
	}
}

fn get_display_resolution() -> Result<(u32, u32)> {
	let output = ProcessCommand::new("swaymsg").args(["-t", "get_outputs"]).output()?;
	let outputs: Vec<SwayOutput> = serde_json::from_slice(&output.stdout)?;
	let output = outputs.iter().find(|o| o.active).context("No active outputs found")?;
	Ok((output.current_mode.width, output.current_mode.height))
}

fn get_all_active_displays() -> Result<Vec<(u32, u32)>> {
	let output = ProcessCommand::new("swaymsg").args(["-t", "get_outputs"]).output()?;
	let outputs: Vec<SwayOutput> = serde_json::from_slice(&output.stdout)?;
	Ok(outputs.iter().filter(|o| o.active).map(|o| (o.current_mode.width, o.current_mode.height)).collect())
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

fn resize_fill(img: image::DynamicImage, target_width: u32, target_height: u32) -> image::DynamicImage {
	use image::{DynamicImage, GenericImageView, imageops};

	let (img_width, img_height) = img.dimensions();
	let img_ratio = img_width as f32 / img_height as f32;
	let target_ratio = target_width as f32 / target_height as f32;

	let (scaled_width, scaled_height) = if img_ratio > target_ratio {
		let scaled_height = target_height;
		let scaled_width = (target_height as f32 * img_ratio) as u32;
		(scaled_width, scaled_height)
	} else {
		let scaled_width = target_width;
		let scaled_height = (target_width as f32 / img_ratio) as u32;
		(scaled_width, scaled_height)
	};

	let resized = img.resize_exact(scaled_width, scaled_height, imageops::FilterType::Lanczos3);

	// Crop from right/bottom (keep left/top aligned) since content typically starts there
	let x_offset = 0;
	let y_offset = 0;

	DynamicImage::ImageRgba8(imageops::crop_imm(&resized.to_rgba8(), x_offset, y_offset, target_width, target_height).to_image())
}

fn generate_text_svg(text: &str, author: Option<&str>, balance: Option<&str>, width: u32, height: u32, safe_area: &SafeArea, text_padding: u32) -> Result<String> {
	// Nested padding levels: [level0, level1, level2, level3, level4]
	// Each level is half of the previous
	let padding_levels: [u32; 5] = [text_padding, text_padding / 2, text_padding / 4, text_padding / 8, text_padding / 16];
	// Escape HTML entities in text
	let escaped_text = text
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;");

	// Calculate text widths (approximate for monospace: char_count * char_width)
	let quote_font_size = 28;
	let char_width_quote = (quote_font_size as f32 * 0.6) as u32; // Monospace chars are ~0.6 of font size
	let quote_lines: Vec<&str> = escaped_text.lines().collect();
	let max_quote_line_len = quote_lines.iter().map(|l| l.len()).max().unwrap_or(0);
	let quote_text_width = max_quote_line_len as u32 * char_width_quote;

	// Position quote in top-right corner of safe area with level 0 padding
	// We use right alignment, so quote_right_edge is the anchor point
	let quote_right_edge = safe_area.x + safe_area.width - padding_levels[0];
	let quote_x = quote_right_edge - quote_text_width;
	let quote_y = safe_area.y + padding_levels[0] * 2;

	// Create tspan elements
	let quote_tspans: String = quote_lines
		.iter()
		.enumerate()
		.map(|(i, line)| {
			if i == 0 {
				format!(r#"<tspan x="{}" dy="0">{}</tspan>"#, quote_x, line)
			} else {
				format!(r#"<tspan x="{}" dy="1.2em">{}</tspan>"#, quote_x, line)
			}
		})
		.collect::<Vec<_>>()
		.join("\n      ");

	// Calculate height of quote block
	let line_height = 34; // 28px * 1.2 ≈ 34
	let quote_height = quote_lines.len() as u32 * line_height;

	// Author is nested inside quote component (level 1 padding)
	let author_y = quote_y + quote_height + padding_levels[1];

	let (author_element, author_height) = if let Some(author) = author {
		let escaped_author = author
			.replace('&', "&amp;")
			.replace('<', "&lt;")
			.replace('>', "&gt;")
			.replace('"', "&quot;")
			.replace('\'', "&apos;");

		// Calculate author text width
		let author_text = format!("© {}", escaped_author);

		// Position author at the same right edge as the quote (right-aligned with text-anchor: end)
		let author_x = quote_right_edge;
		let author_height = 21;
		(format!(r#"<text class="author" x="{}" y="{}">{}</text>"#, author_x, author_y, author_text), author_height)
	} else {
		(String::new(), 0)
	};

	// Calculate the bottom of the quote component (for positioning balance below)
	// Use level 0 padding after the entire quote component
	let quote_bottom_y = if author.is_some() {
		author_y + author_height + padding_levels[0]
	} else {
		quote_y + quote_height + padding_levels[0]
	};

	let balance_element = if let Some(balance) = balance {
		let escaped_balance = balance
			.replace('&', "&amp;")
			.replace('<', "&lt;")
			.replace('>', "&gt;")
			.replace('"', "&quot;")
			.replace('\'', "&apos;");

		// Calculate balance text width
		let balance_font_size = 20;
		let char_width_balance = (balance_font_size as f32 * 0.6) as u32;
		let balance_lines: Vec<&str> = escaped_balance.lines().collect();
		let max_balance_line_len = balance_lines.iter().map(|l| l.len()).max().unwrap_or(0);
		let balance_text_width = max_balance_line_len as u32 * char_width_balance;

		// Position balance right below the quote component (level 0 padding from right edge)
		let balance_x = safe_area.x + safe_area.width - padding_levels[0] - balance_text_width;
		let balance_y = quote_bottom_y;

		// Create tspan elements
		let balance_tspans: String = balance_lines
			.iter()
			.enumerate()
			.map(|(i, line)| {
				if i == 0 {
					format!(r#"<tspan x="{}" dy="0">{}</tspan>"#, balance_x, line)
				} else {
					format!(r#"<tspan x="{}" dy="1.2em">{}</tspan>"#, balance_x, line)
				}
			})
			.collect::<Vec<_>>()
			.join("\n      ");

		format!(
			r#"<text class="balance" x="{}" y="{}">
      {}
  </text>"#,
			balance_x, balance_y, balance_tspans
		)
	} else {
		String::new()
	};

	let svg = format!(
		r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <style>
      .quote {{
        font-family: 'DejaVu Sans Mono';
        font-size: 28px;
        fill: white;
        text-anchor: start;
      }}
      .author {{
        font-family: 'DejaVu Sans Mono';
        font-size: 21px;
        fill: white;
        text-anchor: end;
      }}
      .balance {{
        font-family: 'DejaVu Sans Mono';
        font-size: 20px;
        fill: white;
        text-anchor: start;
      }}
    </style>
  </defs>
  <text class="quote" x="{}" y="{}">
      {}
  </text>
  {author_element}
  {balance_element}
</svg>"#,
		quote_x, quote_y, quote_tspans,
	);

	Ok(svg)
}

fn composite_text_on_image(params: &CompositeParams) -> Result<()> {
	// Load background image
	let mut bg_image = image::open(params.bg_image_path)?.to_rgba8();

	// Generate SVG with just the text elements (no background)
	let svg_content = generate_text_svg(params.text, params.author, params.balance, params.width, params.height, params.safe_area, params.text_padding)?;

	// Set up font database for usvg
	let mut fontdb = fontdb::Database::new();
	fontdb.load_system_fonts();

	// Try to load DejaVu Sans Mono from common locations (for dev environment)
	let dev_font_path = std::env::current_dir().ok().map(|p| p.join("assets/DejaVuSansMono.ttf"));
	if let Some(path) = dev_font_path
		&& path.exists()
	{
		let _ = fontdb.load_font_file(&path); // Ignore errors, system fonts are already loaded
	}

	let options = usvg::Options {
		fontdb: std::sync::Arc::new(fontdb),
		..Default::default()
	};

	let tree = usvg::Tree::from_str(&svg_content, &options)?;

	// Render text SVG to a transparent pixmap
	let mut text_pixmap = tiny_skia::Pixmap::new(params.width, params.height).context("Failed to create pixmap")?;

	resvg::render(&tree, tiny_skia::Transform::default(), &mut text_pixmap.as_mut());

	// Composite text layer onto background image
	for y in 0..params.height {
		for x in 0..params.width {
			let text_pixel = text_pixmap.pixel(x, y).context("Failed to get pixel")?;
			let alpha = text_pixel.alpha();

			if alpha > 0 {
				let bg_pixel = bg_image.get_pixel_mut(x, y);
				let alpha_f = alpha as f32 / 255.0;

				// Alpha blending
				bg_pixel[0] = ((text_pixel.red() as f32 * alpha_f) + (bg_pixel[0] as f32 * (1.0 - alpha_f))) as u8;
				bg_pixel[1] = ((text_pixel.green() as f32 * alpha_f) + (bg_pixel[1] as f32 * (1.0 - alpha_f))) as u8;
				bg_pixel[2] = ((text_pixel.blue() as f32 * alpha_f) + (bg_pixel[2] as f32 * (1.0 - alpha_f))) as u8;
			}
		}
	}

	// Save the composited image
	bg_image.save(params.output_path)?;

	Ok(())
}
