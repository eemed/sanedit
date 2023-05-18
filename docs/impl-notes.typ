#set text(
  font: "New Computer Modern",
  size: 10pt
)
#set heading(numbering: "1.")
#show link: underline
#let pname = "Sanedit"

#text(
  size: 20pt,
  weight: "bold",
[
  #pname Implementation Notes
])

This document contains implementation details and notes. It should provide
answers why certain design decisions were made.

= Crates

#include "impl-notes/buffer.typ"
#include "impl-notes/cli.typ"
#include "impl-notes/editor.typ"
#include "impl-notes/message.typ"
#include "impl-notes/regex.typ"
#include "impl-notes/terminal-client.typ"
#include "impl-notes/ucd.typ"
