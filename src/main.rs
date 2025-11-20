use std::{path::PathBuf, process::Command};

use clap::Parser;
use color_eyre::Result;
use image::GenericImageView;
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
		if let Some(label) = &balance.label {
			println!("{}:\n{}", label, value);
			Some(format!("{}\n{}", label, value))
		} else {
			println!("{}", value);
			Some(value)
		}
	} else {
		None
	};

	println!("Generating CSS...");

	// Get display resolution from swaymsg
	let (display_width, display_height) = get_display_resolution()?;

	// Get all active displays to calculate safe area
	let all_displays = get_all_active_displays()?;
	println!("Found {} active display(s)", all_displays.len());
	for (i, (w, h)) in all_displays.iter().enumerate() {
		println!("  Display {}: {}x{} (ratio: {:.3})", i + 1, w, h, *w as f32 / *h as f32);
	}

	// Save resized background image to temp location
	let temp_bg_path = v_utils::xdg_state_file!("background_temp.png");
	let img = image::open(&input_path)?;
	let resized_img = resize_fill(img, display_width, display_height);
	let (img_width, img_height) = resized_img.dimensions();
	resized_img.save(&temp_bg_path)?;

	// Calculate safe area that will be visible on all monitors
	let safe_area = calculate_safe_area(img_width, img_height, &all_displays);
	println!(
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
	composite_text_on_image(
		&temp_bg_path,
		&output_path,
		&quote.text,
		quote.author.as_deref(),
		balance_text.as_deref(),
		img_width,
		img_height,
		&safe_area,
		text_padding,
	)?;

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
	let output = outputs.iter().find(|o| o.active).ok_or_else(|| color_eyre::eyre::eyre!("No active outputs found"))?;
	Ok((output.current_mode.width, output.current_mode.height))
}

fn get_all_active_displays() -> Result<Vec<(u32, u32)>> {
	let output = Command::new("swaymsg").args(["-t", "get_outputs"]).output()?;
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
	let x_offset = (scaled_width.saturating_sub(target_width)) / 2;
	let y_offset = (scaled_height.saturating_sub(target_height)) / 2;

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
	let quote_x = safe_area.x + safe_area.width - padding_levels[0] - quote_text_width;
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

		// Position author at the end of the longest quote line (right-aligned)
		let author_x = quote_x + quote_text_width;
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

fn composite_text_on_image(
	bg_image_path: &PathBuf,
	output_path: &PathBuf,
	text: &str,
	author: Option<&str>,
	balance: Option<&str>,
	width: u32,
	height: u32,
	safe_area: &SafeArea,
	text_padding: u32,
) -> Result<()> {
	// Load background image
	let mut bg_image = image::open(bg_image_path)?.to_rgba8();

	// Generate SVG with just the text elements (no background)
	let svg_content = generate_text_svg(text, author, balance, width, height, safe_area, text_padding)?;

	// Set up font database for usvg
	let mut fontdb = fontdb::Database::new();
	fontdb.load_system_fonts();

	// Try to load DejaVu Sans Mono from common locations (for dev environment)
	let dev_font_path = std::env::current_dir().ok().map(|p| p.join("assets/DejaVuSansMono.ttf"));
	if let Some(path) = dev_font_path {
		if path.exists() {
			let _ = fontdb.load_font_file(&path); // Ignore errors, system fonts are already loaded
		}
	}

	let mut options = usvg::Options::default();
	options.fontdb = std::sync::Arc::new(fontdb);

	let tree = usvg::Tree::from_str(&svg_content, &options)?;

	// Render text SVG to a transparent pixmap
	let mut text_pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| color_eyre::eyre::eyre!("Failed to create pixmap"))?;

	resvg::render(&tree, tiny_skia::Transform::default(), &mut text_pixmap.as_mut());

	// Composite text layer onto background image
	for y in 0..height {
		for x in 0..width {
			let text_pixel = text_pixmap.pixel(x, y).ok_or_else(|| color_eyre::eyre::eyre!("Failed to get pixel"))?;
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
	bg_image.save(output_path)?;

	Ok(())
}
