use std::{path::PathBuf, process::Command};

use clap::Parser;
use color_eyre::Result;
use rand::seq::SliceRandom;
use serde::Deserialize;
use v_utils::utils::eyre::exit_on_error;
use wallpaper_carousel::config::AppConfig;

#[derive(Debug, Parser)]
#[command(name = "wallpaper_carousel")]
#[command(about = "Extend wallpaper with citation overlays")]
struct Args {
	/// Path to input image file (jpg or png). If not provided, uses the last input file from cache.
	input: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct SwayOutput {
	current_mode: CurrentMode,
}

#[derive(Debug, Deserialize)]
struct CurrentMode {
	width: u32,
	height: u32,
}

fn get_cache_file_path() -> PathBuf {
	v_utils::xdg_cache_file!("last_input.txt")
}

fn save_last_input(path: &PathBuf) -> Result<()> {
	let cache_path = get_cache_file_path();
	if let Some(parent) = cache_path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(&cache_path, path.to_string_lossy().as_bytes())?;
	Ok(())
}

fn load_last_input() -> Result<PathBuf> {
	let cache_path = get_cache_file_path();
	let content = std::fs::read_to_string(&cache_path).map_err(|_| {
		color_eyre::eyre::eyre!(
			"No input file provided and no cached input file found.\n\
			Please provide an input file: wallpaper_carousel <path-to-image>"
		)
	})?;
	Ok(PathBuf::from(content.trim()))
}

fn main() {
	exit_on_error(run());
}

fn run() -> Result<()> {
	color_eyre::install()?;
	let args = Args::parse();

	// Determine input path: use provided arg or load from cache
	let input_path = match args.input {
		Some(path) => path,
		None => load_last_input()?,
	};

	// Load config
	let config = AppConfig::read(None)?;

	// Select a random quote
	let quote = config.quotes.choose(&mut rand::thread_rng()).ok_or_else(|| color_eyre::eyre::eyre!("No quotes configured"))?;
	println!("Selected quote: {:?}", quote.text);
	println!("Author: {:?}", quote.author);

	// Get balance value if configured
	let balance_text = if let Some(balance) = &config.balance {
		let value = balance.get_value()?;
		let label = balance.label.as_deref().unwrap_or("Balance");
		println!("{}: {}", label, value);
		Some(format!("{}: {}", label, value))
	} else {
		None
	};

	println!("Generating CSS...");

	// Get display resolution from swaymsg
	let (display_width, display_height) = get_display_resolution()?;

	// Save resized background image to temp location
	let temp_bg_path = v_utils::xdg_state_file!("background_temp.png");
	let img = image::open(&input_path)?;
	let resized_img = resize_fill(img, display_width, display_height);
	resized_img.save(&temp_bg_path)?;

	// Generate SVG with background image and text overlay
	let svg_content = generate_svg(&temp_bg_path, &quote.text, quote.author.as_deref(), balance_text.as_deref(), display_width, display_height)?;

	// Debug: save SVG for inspection
	let svg_debug_path = v_utils::xdg_state_file!("debug.svg");
	std::fs::write(&svg_debug_path, &svg_content)?;
	println!("SVG saved to {}", svg_debug_path.display());

	// Render SVG to PNG
	let output_path = v_utils::xdg_state_file!("extended.png");
	render_svg_to_png(&svg_content, &output_path, display_width, display_height)?;

	// Set wallpaper using swaymsg
	Command::new("swaymsg").args(["output", "*", "background", output_path.to_str().unwrap(), "fill"]).output()?;

	println!("Wallpaper set to {}", output_path.display());

	// Save the input path to cache for next time
	save_last_input(&input_path)?;

	Ok(())
}

fn get_display_resolution() -> Result<(u32, u32)> {
	let output = Command::new("swaymsg").args(["-t", "get_outputs"]).output()?;
	let outputs: Vec<SwayOutput> = serde_json::from_slice(&output.stdout)?;
	let output = outputs.first().ok_or_else(|| color_eyre::eyre::eyre!("No outputs found"))?;
	Ok((output.current_mode.width, output.current_mode.height))
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
	let x_offset = (scaled_width.saturating_sub(target_width)) / 2;
	let y_offset = (scaled_height.saturating_sub(target_height)) / 2;

	DynamicImage::ImageRgba8(imageops::crop_imm(&resized.to_rgba8(), x_offset, y_offset, target_width, target_height).to_image())
}

fn generate_svg(bg_image_path: &PathBuf, text: &str, author: Option<&str>, balance: Option<&str>, width: u32, height: u32) -> Result<String> {
	// Escape HTML entities in text
	let escaped_text = text
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;");

	// Convert text lines for tspan elements
	let lines: Vec<&str> = escaped_text.lines().collect();

	// Find the longest line for alignment
	let longest_line = lines.iter().max_by_key(|l| l.len()).unwrap_or(&"");
	let quote_x = width / 2 + 40;

	let tspan_elements: String = lines
		.iter()
		.enumerate()
		.map(|(i, line)| format!(r#"<tspan x="{}" dy="{}">{}</tspan>"#, quote_x, if i == 0 { "0" } else { "1.3em" }, line))
		.collect::<Vec<_>>()
		.join("\n      ");

	// Calculate approximate y position for quote text (centered vertically)
	let quote_y = height / 2;

	// Calculate y position for author (below the quote, accounting for number of lines)
	let line_height = 36; // 28px * 1.3 ≈ 36
	let quote_height = lines.len() as u32 * line_height;
	let author_y = quote_y + quote_height + 20; // 20px gap below quote

	let author_element = if let Some(author) = author {
		let escaped_author = author
			.replace('&', "&amp;")
			.replace('<', "&lt;")
			.replace('>', "&gt;")
			.replace('"', "&quot;")
			.replace('\'', "&apos;");
		// Align author to the end of the longest quote line
		// Approximate character width in monospace font at 28px: ~17px per char
		let char_width_quote = 17.0;
		let longest_line_width = longest_line.len() as f32 * char_width_quote;
		let author_x = quote_x as f32 + longest_line_width;
		format!(r#"<text class="author" x="{}" y="{}">{} {}</text>"#, author_x as u32, author_y, "©", escaped_author)
	} else {
		String::new()
	};

	let balance_element = if let Some(balance) = balance {
		let escaped_balance = balance
			.replace('&', "&amp;")
			.replace('<', "&lt;")
			.replace('>', "&gt;")
			.replace('"', "&quot;")
			.replace('\'', "&apos;");

		// Split balance into lines and create tspan elements
		let balance_lines: Vec<&str> = escaped_balance.lines().collect();
		let balance_x = 40;
		let balance_y = 60;

		let balance_tspans: String = balance_lines
			.iter()
			.enumerate()
			.map(|(i, line)| format!(r#"<tspan x="{}" dy="{}">{}</tspan>"#, balance_x, if i == 0 { "0" } else { "1.3em" }, line))
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
<svg width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
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
        white-space: pre;
      }}
    </style>
  </defs>
  <image href="file://{}" x="0" y="0" width="{width}" height="{height}" />
  {balance_element}
  <text class="quote" x="{}" y="{}">
      {tspan_elements}
  </text>
  {author_element}
</svg>"#,
		bg_image_path.display(),
		width / 2 + 40,
		quote_y,
	);

	Ok(svg)
}

fn render_svg_to_png(svg_content: &str, output_path: &PathBuf, width: u32, height: u32) -> Result<()> {
	// Set up font database for usvg
	let mut fontdb = fontdb::Database::new();
	fontdb.load_system_fonts();

	// Load our custom font
	let font_path = std::env::current_dir()?.join("assets/DejaVuSansMono.ttf");
	fontdb.load_font_file(&font_path)?;

	let mut options = usvg::Options::default();
	options.fontdb = std::sync::Arc::new(fontdb);

	let tree = usvg::Tree::from_str(svg_content, &options)?;

	let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| color_eyre::eyre::eyre!("Failed to create pixmap"))?;

	resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

	pixmap.save_png(output_path)?;

	Ok(())
}
