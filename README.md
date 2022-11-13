# sanedit

Text editor to learn about text buffers, syntax highlighting, regex (automata),
client-server architecture (using tokio) and data structures (red-black tree).

The editor uses client server architecture. Each client is an editor window, and
managing the windows is left to the operating systems window manager (see
kakoune).

Inspiration taken from vim, kakoune, emacs, neovim and vis.

## Crates

### buffer

Text buffer implementation. It is a piece tree a piece table variant
which stores the pieces in a red-black tree instead of a vec. To understand what
a piece tree is lets start with a piece table.

Piece table uses two text buffers, original buffer which contains the original
text and an add buffer which contains added text. The original buffer is read
only and the add buffer is append only. This means that you can index into these
buffers as the text never changed. Piece table stores pieces which are an offset +
length pair. The pieces then reference the buffers and form the buffer contents.
This means the buffer content is not contiguous in memory.
[For more information on piece tables](https://en.wikipedia.org/wiki/Piece_table)

Then a piece tree is a piece table that stores the pieces in a red-black tree.
The nodes contain left subtree length (in bytes) so we can search for a bytes
offset in O(log(n)) where n is the number of pieces in the tree. Rust provides
easy access to cow functionality using `Rc::make_mut` which is used in the tree.
It allows us to take lightweight snapshots of the tree, which can then be
restored. This is a built in solution for undoing changes. The snapshots are
lightweight as the trees can share nodes, new nodes are only created if
snapshots referencing the node still exist.

For more information on piece trees see [Improving the AbiWord's Piece Table](http://e98cuenc.free.fr/wordprocessor/piecetable.html),
[VS Code Text Buffer Reimplementation](https://code.visualstudio.com/blogs/2018/03/23/text-buffer-reimplementation)


This implementation operates on bytes and does not guarantee that the content is
valid UTF-8. The bytes are decoded to UTF-8 when needed. Line counts could also
be cached to nodes (like left subtree byte length) to provide a fast line
search, but this implementation opts of that so we do not need to read the whole
file on load. This means opening large files should be as fast as opening small
ones.

#### Piece tree is good at

* Random insertion and deletion
* Large files
* Easy undo implementation
* Easy text position tracking by indexing into backing buffers


#### Piece tree is bad at

* Sequential insert and delete are not O(1) amortized time.
* Performance deteriorates over time as more pieces are in the tree
* Complex implementation because of red-black trees

### cli

Commandline interface to launch the editor.

### editor

Editor implementation itself.

### messages

Messages sent between editor and clients.

### terminal-client

Terminal client for the editor. Its job is to send keyboard and mouse events to
the editor and draw the screen.

### view

Used to draw editor state.

