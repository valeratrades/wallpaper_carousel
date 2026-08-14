// Overlay payload is handed in whole by the caller as `--input overlay=<json>`.
#let data = json(bytes(sys.inputs.at("overlay", default: "{}")))

#let page-size = (
  width: data.at("width", default: 1920) * 1pt,
  height: data.at("height", default: 1080) * 1pt,
)

#let _inset = data.at("inset", default: (:))
/// Extra margin so nothing lands outside the region visible on every display.
#let inset = (
  top: _inset.at("top", default: 0) * 1pt,
  bottom: _inset.at("bottom", default: 0) * 1pt,
  left: _inset.at("left", default: 0) * 1pt,
  right: _inset.at("right", default: 0) * 1pt,
)

// quotes in the config are hand-wrapped
#let _lines(s) = s.split("\n").join(linebreak())

#let overlay() = align(right)[
  #set text(font: "DejaVu Sans Mono", fill: white)
  #set par(justify: false)
  #text(size: 28pt, _lines(data.at("quote", default: "")))
  #let author = data.at("author", default: none)
  #if author != none {
    linebreak()
    text(size: 21pt, "© " + author)
  }
  #for stat in data.at("stats", default: ()) {
    block(above: 20pt, text(size: 20pt, _lines(stat)))
  }
]
