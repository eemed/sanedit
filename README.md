# sanedit

Text editor to learn about text buffers, syntax highlighting, regex (automata),
client-server architecture (using tokio) and data structures (red-black tree).

The editor uses client server architecture. Each client is an editor window, and
managing the windows is left to the operating systems window manager (see
kakoune).

Inspiration taken from vim, kakoune, emacs, neovim and vis.

## TODO

* Better regex support
* Multicursor
* Search multiple matches
* Search next/prev match
* Locations on a list
* Undo
* Saving (copy from eemedit)
* Pairs (copy from eemedit)
* Syntax highlighting using pest

* Shell access

### Later

* Prompt complete funcs, not just pick from precomputed options but actually
  complete the term
* File picker performance?
