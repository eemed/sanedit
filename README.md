# SanEdit

## TODO

* LSP stuff menu
    * Restart server
    * Stop server
    * all LSP actions if running

* Grepping can match / display very long lines, this should be limited to X
  characters at most. For example show the non matched 20 first characters and the
  matched text afterwards with combined limit to 300 chars
* Keybindings should be sensical
    * Line up down char next prev, to keyboard not arrow keys
    * Keybinds to configuration file => no default in code
* Somethings wrong with multiwidth grapheme rendering.
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

