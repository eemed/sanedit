# SanEdit

## TODO

* core dirs has todos
* Somethings wrong with multiwidth grapheme rendering. Grapheme widths is wrong
* Keybindings should be sensical
    * Mouse 2 click select word
    * 3 click select line
    * ctrl shift (left/right) => extend selection to
    * shift (left/right) => goto next word
    * Home start of line
    * End end of line

* Welcome screen
    * Help menus to F1

* Save in place
* Async saving
    - Start saving if modified while saving quit the save process => prompt this so no accidental cancellations can occur
* play around with parsing machine JIT
    - This is its own big project
    - Should try this once we can write it in this editor :)

* Basic highlighting for jsx, yang, java, xml, makefile, dockerfile
    - injection of other languages in peg

## Tests

* Using the thing to see whats wrong
    - Should write tests for these fixes so that they do not reappear
* unicode grapheme boundary test set


## Future

* Hex mode
* Snippets
    - a word prefix + key to expand 
    - => Tab behaviour 
    1. insert indent if in indentation
    - if word before cursor
    1. Expand snippet
    2. word completion

