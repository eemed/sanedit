= Editor

Editor implementation itself.

The editor server accepts connections through TCP and unix domain sockets. The
connections are considered to be a single window in the editor. Messages that
are passed from the client to the server are described in @messages.

== Structure

The editor is structed into a few subdirectories.


#table(
  columns: (auto, auto),
  inset: 10pt,
  align: horizon,
  [*Directory*], [*Usage*],
  [common],  [functions that do not directly change the editor state],
  [actions], [functions that directly change the editor state],
  [draw],    [used to draw all the things for the client],
  [editor],  [holds the editor state model and functions to ease operating on it],
  [server],  [handles async with tokio, handles all the client connections, jobs, etc.]
)


== Threading

The editor itself is run on a single thread and tokio runtime is used along with
channels to provide async functionality. Async is used to accept and handle the
client connections and to run jobs that the editor requests.

#figure(
  image("../images/threading.png", width: 100%),
  caption: [Editor threading.],
) <threading>

== Jobs

The editor can ask tokio to run jobs. Jobs are just functions that can produce
gradual output. Jobs were created to run long running processes in the
background whilst having a responsive editor. Best usages for jobs are shell
commands, large buffer saving, listing files can be implemented as jobs.

