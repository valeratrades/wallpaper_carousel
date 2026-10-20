#import "@local/wallpaper-carousel:0.1.0": inset, overlay, page-size

#set page(
  width: page-size.width,
  height: page-size.height,
  fill: rgb("#333333"),
  margin: inset,
  columns: 2,
)
#set text(size: 12.5pt, fill: white)
#set columns(gutter: 40pt)

= vision
#lorem(400)

== tangible
#lorem(250)

#colbreak()
#lorem(150)

#overlay()
