= Syntax <parser>

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


==== Error Recovery

Syntax error recovery using PEGs is hard.
Parsing machine naturally fails when trying
different choices to match. One option is to detect fails which should be
corrected using recovery expresssions (other PEG expressions). These failure
lables could be inserted by hand, but this is error prone. Some algorithms also
exist to automatically insert these labels, but they usually add too many or too
little labels.

Another option is to let the parsing machine fail completely, when it encounters a syntax
error and keep track of the longest position we were succesfully able to parse.
This reliably finds syntax errors, but erases all of the parser state required
to restart the parsing process after corrections. Copying the parser state
(stack) is rather expensive and would slow down performance.


== Current solution

Wrote a LPEG like parsing machine and a JIT to version.

The grammars are matching parts of the structure instead of the whole
structure, to allow most things to be highlighted while we are writing
code. The current implementation supports injecting other languages into other languages recursively. For example fenced code blocks can be highlighted in their own syntax, that in turn can contain other injected languages. The implementation also supports backreferences so you can match previously matched text again, useful for example in HTML or XML.

=== Defining rules

Rules are specified in special files called `syntax.peg`. They use a simple PEG like syntax with custom annotations. Special rules are capitalized.

```peg
document = (!skip .)* (member / value / .)*;

WHITESPACE = [ \t\r\n];
_ = WHITESPACE*;

skip = "{" / "[";
value =  object / array / number / string / constant;

@whitespaced
object = "{" (member ("," member)*)? "}"?;

@whitespaced
member = identifier ":" value;

@show @highlight(identifier)
identifier = quote identifier_inner quote;

@show @completion
identifier_inner = (escape / !"\"" .)*;
escape = "\\" .;

@show @highlight
string = quote string_inner quote;

@show @completion @highlight(string)
string_inner = (escape / !"\"" .)*;
escape = "\\" .;

@whitespaced
array = "[" (value ("," value)*)? "]";

@show @highlight(constant)
number = "-"? integer ("." [0..9]+)?;

@show @highlight
constant = true / false / null;

true = "true";
false = "false";
null = "null";
integer = "0" / [1..9] [0..9]*;
quote = "\"";
```

Supported annotations are:

#table(
  columns: (auto, auto),
  inset: 10pt,
  align: horizon,
  table.header(
    [*Annotation*], [*Usage*],
  ),
  [\@show],                [Show the rule in captures],
  [\@highlight],           [Highlight text matched by this rule],
  [\@highlight(anything)], [Highlight text matched by this rule, and use the <anything> provided as a name],
  [\@completion],          [Use the text matched by this rule as a completion item],
  [\@static-completion],   [Use the strings defined in this rule directly as completion items],
  [\@injection-language],  [Matches the dynamic name of the language to be injected],
  [\@inject],              [Injects previously matched \@injection-language to this portion of text],
  [\@inject(javascript)],  [Injects static language name to a portion of text],
 [\@whitespaced],          [Convert the rule to a form that allows whitespace in between every element. \
                 a special WHITESPACE rule defines what is considered whitespace. \
                 Example:\
                     Original:   array = "\["             value (              "\,"            value )\*             "\]";\
                     Generated:  array = "\[" WHITESPACE\* value ( WHITESPACE\* "\," WHITESPACE\* value )\* WHITESPACE\* "\]";],
)

Supported inline annotations are:

#table(
  columns: (auto, auto),
  inset: 10pt,
  align: horizon,
  table.header(
    [*Inline annotation*], [*Usage*],
  ),
  [\@backref(rule_name)],  [References the same text as rule_name previously matched],
)
