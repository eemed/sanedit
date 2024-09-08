# SanEdit

## TODO

* LSP
    - Completion
        - needs documentation
        - needs type
    - Diagnostics: Somehow to show them
        - Not done at all


* Somethings wrong with multiwidth grapheme rendering.
* Keybinds to configuration file => no default
* Async saving
    - Start saving if modified while saving quit the save process => prompt this so no accidental cancellations can occur
* Welcome screen
* Help menus to F1

* Keybindings should be sensical
    * Line up down char next prev, to keyboard not arrow keys
* Save in place

* Grepping can match / display very long lines, this should be limited to X
  characters at most. For example show the non matched 20 first characters and the
  matched text afterwards with combined limit to 300 chars

* play around with parsing machine JIT
    - This is its own big project
    - Should try this once we can write it in this editor :)

* Add log buffer directly to editor

* Basic highlighting for jsx, yang, java


## Future

* LSP
    * Needs installation mechanism => complex
* Snippets
    - a word prefix + key to expand 
    - => Tab behaviour 
    1. insert indent if in indentation
    - if word before cursor
    1. Expand snippet
    2. word completion

