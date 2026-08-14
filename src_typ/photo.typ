#import "overlay.typ": data, inset, overlay, page-size

#set page(
  width: page-size.width,
  height: page-size.height,
  margin: (
    top: inset.top + 20pt,
    bottom: inset.bottom + 20pt,
    left: inset.left + 20pt,
    right: inset.right + 20pt,
  ),
  background: image(data.bg, width: 100%, height: 100%, fit: "cover"),
)

#place(top + right, block(width: 55%, overlay()))
