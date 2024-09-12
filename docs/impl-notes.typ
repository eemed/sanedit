#let pname = "Sanedit"

// Some space for headings
#show heading: name => box[
  #v(0.2em)
  #name
  #v(0.2em)
]
#set heading(numbering: "1.1")

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
answers what was the thought process behind certain design decisions.

= Project structure

#table(
  columns: (auto, auto),
  inset: 10pt,
  align: horizon,
  [*Crate*], [*Description*],
  [buffer],  [Text buffer implementation],
  [cli], [Commandline argument handling, starting the editor],
  [core],    [Core structures and functionality],
  [editor],  [Editor state using core structures and functions to modify editor state],
  [lsp],  [LSP client implementation],
  [macros],  [Macros to easily create functions to modify editor state],
  [server],  [Server to talk to clients (client per window) and to send events to/from editor],
  [syntax],  [Syntax higlighting using LPEG like parsing machine],
  [terminal-client],  [A terminal client to talk to server and display editor],
  [ucd],  [Unicode character database for unicode handling],
  [utils],  [Useful functions and structures used in multiple places],
)

#include "impl-notes/buffer.typ"

#include "impl-notes/cli.typ"

#include "impl-notes/editor.typ"

#include "impl-notes/message.typ"

#include "impl-notes/terminal-client.typ"

#include "impl-notes/ucd.typ"

#include "impl-notes/parser.typ"
