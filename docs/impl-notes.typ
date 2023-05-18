#let pname = "Sanedit"

// Some space for headings
#show heading: name => box[
  #v(0.2em)
  #name
  #v(0.2em)
]

// Blue undelined links
#show link: name => box[
  #text(fill: blue, )[
    #underline[#name]
  ]
]

// Darker blocks
#show raw.where(block: false): box.with(
  fill: luma(225),
  inset: (x: 3pt, y: 0pt),
  outset: (y: 3pt),
  radius: 2pt,
)

#text(
  size: 26pt,
  weight: "bold",
[
  #pname Implementation Notes
])

This document contains implementation details and notes. It should provide
answers why certain design decisions were made.

#include "impl-notes/buffer.typ"

#include "impl-notes/cli.typ"

#include "impl-notes/editor.typ"

#include "impl-notes/message.typ"

#include "impl-notes/regex.typ"

#include "impl-notes/terminal-client.typ"

#include "impl-notes/ucd.typ"
