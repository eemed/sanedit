![SanEdit Logo](https://github.com/user-attachments/assets/b75335be-d847-4ca3-a730-7314db4ee031)

---

SanEdit is a UTF-8 text editor inspired by [Neovim](https://neovim.io/), [Emacs](https://www.gnu.org/software/emacs/), [Kakoune](https://kakoune.org/) and [Helix](https://helix-editor.com/).

## Features

* Any file size
* Multiple cursors
* Filetree
* Terminal UI
* Snippets
* Comment block
* Search text
* File picker
* Undo, redo
* Align cursors
* Automatic indentation
* Automatic pair insertion on enter
* Syntax highlighting using JPEGs
    * Regex and glob patterns are handled by converting them to PEGs
    * JIT compiler for PEG patterns to improve performance
* Jumps
    * to last change
    * to last cursor position
    * to last file
* Language server protocol support
    * Hover information
    * Go to definition
    * Find references
    * Completion
    * Code diagnostics
    * Rename variable
    * Format
    * Code actions

## Planned features

* GUI using non web technologies
* More syntax highlighted languages

## Building and running

The project is built using a [Taskfile](https://taskfile.dev/installation/) and [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html), install them and use the command `task` to see all available commands and their descriptions.

To quickly run the project in the repo run `task run`.

## Why

There are more than enough editors out there and this one was born because of my
initial interest in text buffer implementations. Most prominent question was why
no editor could support large files in any meaningful way. Logically I also
created an editor to go with the buffer implementation.

## More information

Checkout the repositories docs folder for scribblings on why certain decisions
were made or how they are implemented.
