#import "overlay.typ": data, inset, overlay, page-size

#set page(
  width: page-size.width,
  height: page-size.height,
  margin: inset,
  background: image(data.bg, width: 100%, height: 100%, fit: "cover"),
)

#place(top + right, block(width: 55%, overlay()))
