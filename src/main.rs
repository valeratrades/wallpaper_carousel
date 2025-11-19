use std::{path::PathBuf, process::Command};

use ab_glyph::{FontVec, PxScale};
use clap::Parser;
use color_eyre::Result;
use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage, imageops};
use imageproc::drawing::draw_text_mut;
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(name = "wallpaper_carousel")]
#[command(about = "Extend wallpaper with citation overlays")]
struct Args {
	/// Path to input image file (jpg or png)
	input: PathBuf,
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

fn main() -> Result<()> {
	color_eyre::install()?;
	let args = Args::parse();

	// Get display resolution from swaymsg
	let (display_width, display_height) = get_display_resolution()?;

	// Load the input image
	let img = image::open(&args.input)?;

	// Detect the image format from the input file
	let format = ImageFormat::from_path(&args.input)?;
	let extension = match format {
		ImageFormat::Png => "png",
		ImageFormat::Jpeg => "jpg",
		_ => "png", // Default to png for other formats
	};

	// Resize image to fill display (like swaymsg does)
	let resized_img = resize_fill(img, display_width, display_height);
	let mut rgba_img = resized_img.to_rgba8();

	// Add citation text at the rightmost middle point
	add_citation(&mut rgba_img, "hello world");

	// Save to XDG_STATE_HOME
	let output_path = v_utils::xdg_state_file!(&format!("extended.{}", extension));
	rgba_img.save(&output_path)?;

	// Set wallpaper using swaymsg
	Command::new("swaymsg").args(["output", "*", "background", output_path.to_str().unwrap(), "fill"]).output()?;

	println!("Wallpaper set to {}", output_path.display());
	Ok(())
}

fn get_display_resolution() -> Result<(u32, u32)> {
	let output = Command::new("swaymsg").args(["-t", "get_outputs"]).output()?;

	let outputs: Vec<SwayOutput> = serde_json::from_slice(&output.stdout)?;

	// Get the first active output
	let output = outputs.first().ok_or_else(|| color_eyre::eyre::eyre!("No outputs found"))?;

	Ok((output.current_mode.width, output.current_mode.height))
}

fn resize_fill(img: DynamicImage, target_width: u32, target_height: u32) -> DynamicImage {
	let (img_width, img_height) = img.dimensions();
	let img_ratio = img_width as f32 / img_height as f32;
	let target_ratio = target_width as f32 / target_height as f32;

	// Calculate dimensions to fill the target while maintaining aspect ratio
	let (scaled_width, scaled_height) = if img_ratio > target_ratio {
		// Image is wider than target, scale to height
		let scaled_height = target_height;
		let scaled_width = (target_height as f32 * img_ratio) as u32;
		(scaled_width, scaled_height)
	} else {
		// Image is taller than target, scale to width
		let scaled_width = target_width;
		let scaled_height = (target_width as f32 / img_ratio) as u32;
		(scaled_width, scaled_height)
	};

	// Resize the image
	let resized = img.resize_exact(scaled_width, scaled_height, imageops::FilterType::Lanczos3);

	// Crop to exact target dimensions (center crop)
	let x_offset = (scaled_width.saturating_sub(target_width)) / 2;
	let y_offset = (scaled_height.saturating_sub(target_height)) / 2;

	DynamicImage::ImageRgba8(imageops::crop_imm(&resized.to_rgba8(), x_offset, y_offset, target_width, target_height).to_image())
}

fn add_citation(img: &mut RgbaImage, text: &str) {
	let (width, height) = img.dimensions();

	// Embed DejaVu Sans Mono font
	let font_data = include_bytes!("../assets/DejaVuSansMono.ttf");
	let font = FontVec::try_from_vec(font_data.to_vec()).expect("Error loading font");

	let scale = PxScale::from(32.0);
	let color = Rgba([255u8, 255u8, 255u8, 255u8]); // White text

	// Calculate text position (rightmost middle)
	// We'll position it with some padding from the right edge
	let _padding = 20;
	let y_position = (height / 2) as i32;

	// For right alignment, we need to measure the text width
	// For now, we'll use a simple approach and place it near the right edge
	let x_position = (width - 200) as i32; // Approximate positioning

	draw_text_mut(img, color, x_position, y_position, scale, &font, text);
}
