Extend the wallpaper with a quote, a balance readout, billionaire trivia and a year-progress bar.

### Vision Document
Following [Charlie Morgan](https://youtu.be/IUxn7vT104Y?si=xXYJEDzycHf0l8Lu), defining exactly where I must be.

Most importantly, should be actively used for decision-making. What would the target version of myself do? If answer is known, - must follow it. The more difficult to follow, the better, - means that's likely the discrepancy.

`generate` renders the typst document at `vision_source` (required in config). The document is yours; it lays the overlay into its own flow by importing it, see [examples/vision.typ](./examples/vision.typ):
```typst
#import "@local/wallpaper-carousel:0.1.0": inset, overlay, page-size
```
The nix package puts it on `TYPST_PACKAGE_PATH`. Outside nix, link `src_typ/` to `~/.local/share/typst/packages/local/wallpaper-carousel/0.1.0`.
