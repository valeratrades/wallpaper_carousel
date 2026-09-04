// Overlay payload is handed in whole by the caller as `--input overlay=<json>`.
#let data = json(bytes(sys.inputs.at("overlay", default: "{}")))

#let page-size = (
  width: data.at("width", default: 1920) * 1pt,
  height: data.at("height", default: 1080) * 1pt,
)

#let _inset = data.at("inset", default: (:))
/// Page margin: the region invisible on some display, plus `edge_padding`.
#let inset = (
  top: _inset.at("top", default: 0) * 1pt,
  bottom: _inset.at("bottom", default: 0) * 1pt,
  left: _inset.at("left", default: 0) * 1pt,
  right: _inset.at("right", default: 0) * 1pt,
)

/// Chart rows are drawn in `FillLevels`, which owns the private-use block from `0xF09E5` up.
#let _chart-row(line) = line != "" and line.codepoints().all(c => str.to-unicode(c) >= 0xF09E5)

// quotes in the config are hand-wrapped
#let _lines(s) = {
  let lines = s.split("\n")
  // chart glyphs span the whole ascender-to-descender box, so any leading between rows tears the chart apart
  let stacked(i, line) = context block(
    spacing: if i > 0 and _chart-row(line) and _chart-row(lines.at(i - 1)) { 0pt } else { par.leading },
    line,
  )
  lines.enumerate().map(p => stacked(..p)).join()
}

#let overlay() = align(right)[
  // `FillLevels` is named explicitly: several installed fonts claim the same private-use block, and automatic fallback picks among them per run
  #set text(font: ("DejaVu Sans Mono", "FillLevels"), fill: white)
  #set par(justify: false)
  #text(size: 28pt, _lines(data.at("quote", default: "")))
  #let author = data.at("author", default: none)
  #if author != none {
    text(size: 21pt, context block(spacing: par.leading, "© " + author))
  }
  #for stat in data.at("stats", default: ()) {
    block(above: 20pt, text(size: 20pt, _lines(stat)))
  }
]
