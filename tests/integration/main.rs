//! Entry point to all integration tests, following https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html

use std::{path::PathBuf, process::Command};

/// Number of pages `vision.typ` renders for the given overlay payload.
fn render_pages(name: &str, overlay: serde_json::Value) -> usize {
	let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let stem = std::env::temp_dir().join(format!("wallpaper_carousel_{name}"));
	for n in 1..=9 {
		let _ = std::fs::remove_file(format!("{}{n}.png", stem.display()));
	}

	let out = Command::new("typst")
		.args(["compile", "--format", "png", "--root", "/", "--ppi", "72", "--input"])
		.arg(format!("overlay={overlay}"))
		.arg(root.join("src_typ/vision.typ"))
		.arg(format!("{}{{n}}.png", stem.display()))
		.output()
		.expect("typst on PATH");
	assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

	(1..=9).take_while(|n| PathBuf::from(format!("{}{n}.png", stem.display())).exists()).count()
}

fn payload(stats: serde_json::Value) -> serde_json::Value {
	serde_json::json!({
		"quote": "good intentions don't work\nmechanisms do.",
		"author": "Jeff Bezos",
		"stats": stats,
		"width": 1920,
		"height": 1080,
		"inset": { "top": 15, "bottom": 15, "left": 110, "right": 110 },
	})
}

/// The payload shape is the contract between `main.rs` and `overlay.typ`; a field renamed on either side breaks here.
#[test]
fn vision_fits_one_page() {
	let blurb = "Sergei Kolesnikov co-founded Technonicol in 1992 with Igor Rybakov, building a global manufacturing company producing roofing, waterproofing, \
		and thermal insulation materials. Operating 72 factories across Europe and Asia, Kolesnikov owns 50% of the enterprise.";
	assert_eq!(render_pages("fits", payload(serde_json::json!(["balance\n$1,234.56", "3428 billionaires", blurb]))), 1);
}

#[test]
fn overflowing_overlay_spills_to_a_second_page() {
	assert!(render_pages("overflow", payload(serde_json::json!(vec!["a ".repeat(400); 4]))) > 1);
}
