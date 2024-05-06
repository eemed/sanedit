= Parser <parser>

Requirements:

1. Easy to modify at runtime
2. Fast enough to parse on every keystroke
3. Easy to maintain grammars


== Why not one of the existing solutions

Regex syntax highlighters are not easy to maintain or understand.

Tree sitter is
good but we need support for larger than memory files too also don't like the
hassle of compiling parsers for individual languages. I want to change the
grammar at runtime and see changes immediately.

Luas LPEG library would be a good fit, but I don't want to include a language
just for one library.


== PEG

Decided to implement a parser using PEG rules.


== Parsing

=== Pikaparsing

The first parsing strategy tried was pikaparsing
#link("https://arxiv.org/pdf/2005.06444")[pikaparsing].

Error recovery was very good, as we could easily see the best match at a given position.
The memory usage was too high and performance was not that good.


=== LPEG & GPEG

LPEG library was a good fit and more performant than pikaparsing. Also
found an improved version of LPEG called
#link("https://zyedidia.github.io/notes/yedidia_thesis.pdf")[GPEG] that also
supports incremental parsing. This is a good reference point for our own
implementation of something similar.

The only weakness is error recovery. It is theorized in the paper but not implemented.

==== UTF8

Byte sequences can naturally represent also UTF8.
Also UTF8 Ranges are added to ease usage for example to match any UTF8
characters `[\u0000..\u10ffff]`.

`\u` enables UTF8 support: If an UTF8 point or range is used in negated sets `[^\u0020]` the negated set is
expanded to `[\u0000..\u0019] / [\u0021..\u10ffff]`, but the same negated set if
written `[^\x20]` is expanded to `[\x00..\x19] / [\x21..\xff]`

Note that all the sets are inclusive.

