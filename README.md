
> [!WARNING]
> works exclusively on `sway`, and is only meant for me and myself.
# wallpaper_carousel
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-2026--06--29+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/wallpaper_carousel.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/wallpaper_carousel)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/wallpaper_carousel)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/wallpaper_carousel-loc.json)
![Build time](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/wallpaper_carousel-build.json)
![Test time](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/wallpaper_carousel-test.json)
<br>
[![errors](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/wallpaper_carousel-errors.json)](https://github.com/valeratrades/wallpaper_carousel/actions/workflows/errors.yml?query=branch%3Amain)
[![warnings](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/wallpaper_carousel-warnings.json)](https://github.com/valeratrades/wallpaper_carousel/actions/workflows/warnings.yml?query=branch%3Amain)

Extend the wallpaper with a quote, a balance readout, billionaire trivia and a year-progress bar.

#### Vision Document
Following [Charlie Morgan](https://youtu.be/IUxn7vT104Y?si=xXYJEDzycHf0l8Lu), defining exactly where I must be.

Most importantly, should be actively used for decision-making. What would the target version of myself do? If answer is known, - must follow it. The more difficult to follow, the better, - means that's likely the discrepancy.

`generate` renders the typst document at `vision_source` (required in config). The document is yours; it lays the overlay into its own flow by importing it, see [examples/vision.typ](./docs/.readme_assets/examples/vision.typ):
```typst
#import "@local/wallpaper-carousel:0.1.0": inset, overlay, page-size
```
The nix package puts it on `TYPST_PACKAGE_PATH`. Outside nix, link `src_typ/` to `~/.local/share/typst/packages/local/wallpaper-carousel/0.1.0`.
<!-- markdownlint-disable -->
<details>
<summary>
<h2>Installation</h2>
</summary>

TODO


</details>
<!-- markdownlint-restore -->

## Usage
TODO

# Other
NB: note to self, - do not `git lfs` it, - nixos build won't be able to deref


<br>

#### Dev links
- <sup><a href="./docs/ARCHITECTURE.md">architecture</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/blob/main/code/STYLE.md">style</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/tree/main/code/conventions">conventions</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/blob/main/code/conventions/git">contributing</a></sup>

#### License

<sup>
	Licensed under <a href="LICENSE">GLWTS</a>
</sup>

<br>

<sub>
	Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be licensed as above, without any additional terms or conditions.
</sub>

