mod logging;

use std::{
    fs,
    io::Read as _,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use argh::FromArgs;
use sanedit_server::{Address, ServerOptions};
use sanedit_terminal_client::{unix::UnixDomainSocketClient, ClientOptions, InitialFile};

const SESSION_NAMES: [&str; 10] = [
    "wolf",
    "tiger",
    "lion",
    "ghost",
    "pidgeon",
    "bunny",
    "diamond",
    "scarecrow",
    "wheat",
    "wine",
];

struct Session {
    name: String,
    socket: PathBuf,
}

impl Session {
    pub fn new(sessions: &Path, name: &str) -> Session {
        let fname = format!("{name}.sock");
        let socket = sessions.join(fname);
        Session {
            name: name.into(),
            socket,
        }
    }
}

/// command line options
#[derive(Debug, FromArgs)]
#[argh(help_triggers("-h", "--help", "help"))]
struct Cli {
    /// file to open
    #[argh(positional)]
    file: Option<PathBuf>,

    /// read from stdin
    #[argh(switch)]
    stdin: bool,

    /// turn debugging information on
    #[argh(switch)]
    debug: bool,

    /// set configuration directory
    #[argh(option)]
    config_dir: Option<PathBuf>,

    /// set working directory
    #[argh(option)]
    working_dir: Option<PathBuf>,

    /// session to connect or create
    #[argh(option, short = 's')]
    session: Option<String>,

    /// set language for file provided / read from stdin
    #[argh(option)]
    language: Option<String>,

    /// parent client, used to inherit options
    #[argh(option, hidden_help)]
    parent_client: Option<usize>,

    /// create server only, no UI
    #[argh(switch)]
    server_only: bool,

    /// print available sessions
    #[argh(switch, short = 'l')]
    list_sessions: bool,

    /// print version
    #[argh(switch, short = 'v')]
    version: bool,
}

fn main() {
    let tmp = sanedit_core::tmp_dir().expect("TMP directory not accessible");
    let log_tmp = {
        let log_tmp = tmp.join("log");
        std::fs::create_dir_all(&log_tmp).expect("Failed to create log directory");
        log_tmp
    };
    let sessions = {
        let sessions = tmp.join("session");
        std::fs::create_dir_all(&sessions).expect("Failed to create sessions directory");
        sessions
    };
    let cli: Cli = argh::from_env();

    if cli.version {
        print_version();
        return;
    }

    if cli.list_sessions {
        list_sessions(&sessions);
        return;
    }

    let session = cli
        .session
        .as_ref()
        .map(|name| Session::new(&sessions, name))
        .unwrap_or_else(|| next_available_session(&sessions));
    let log_file = init_logging(&cli, &log_tmp, &session);
    let client_opts = ClientOptions {
        file: file_to_open(&cli),
        parent_client: cli.parent_client,
        session: session.name.clone(),
        language: cli.language.clone(),
    };
    let existing_session = session.socket.try_exists().unwrap_or(false);
    let server_opts = ServerOptions {
        config_dir: cli.config_dir.clone(),
        working_dir: working_dir(&cli),
        debug: cli.debug,
        addr: Address::UnixDomainSocket(session.socket.clone()),
    };

    if cli.server_only {
        start_server(server_opts);
        let _ = fs::remove_file(session.socket);
    } else if existing_session {
        // Try to connect if it doest work create a new one
        if let Some(opts) = connect_to_socket(&session.socket, client_opts) {
            start_server_process(&cli, &session, server_opts);
            connect_to_socket(&session.socket, opts);
        }
    } else {
        start_server_process(&cli, &session, server_opts);
        connect_to_socket(&session.socket, client_opts);
    }

    if let Some(log_file) = log_file {
        let _ = fs::remove_file(&log_file);
    }
}

fn working_dir(cli: &Cli) -> Option<PathBuf> {
    cli.file
        .as_ref()
        .and_then(|f| f.canonicalize().ok())
        .and_then(|f| if f.is_dir() { Some(f.clone()) } else { None })
        .or(cli.working_dir.clone())
}

fn file_to_open(cli: &Cli) -> Option<InitialFile> {
    if cli.stdin {
        let mut buf = vec![];
        let _ = std::io::stdin().read_to_end(&mut buf);
        Some(InitialFile::Stdin(buf))
    } else {
        let path = cli.file.as_ref()?;
        let path = path.canonicalize().ok().unwrap_or(path.clone());
        if !path.is_dir() {
            Some(InitialFile::Path(path.clone()))
        } else {
            None
        }
    }
}

fn print_version() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("{VERSION}");
}

fn list_sessions(sessions: &Path) {
    println!("Available sessions:");
    let mut paths = fs::read_dir(sessions).unwrap();
    while let Some(Ok(path)) = paths.next() {
        let path = path.path();
        if let Some(fname) = path.file_name() {
            let name = fname.to_string_lossy();
            println!("{}", &name[..name.len() - ".sock".len()]);
        }
    }
}

fn init_logging(cli: &Cli, tmp: &Path, session: &Session) -> Option<PathBuf> {
    let log_file = next_available_log_file(tmp, cli, session);
    logging::init_panic();
    logging::init_logger(&log_file, cli.debug);

    Some(log_file)
}

fn next_available_log_file(tmp: &Path, cli: &Cli, session: &Session) -> PathBuf {
    let mut i = 0;
    loop {
        let mut id = String::new();
        if i != 0 {
            id = format!("-{i}");
        }
        let mut client = String::new();
        if !cli.server_only {
            client = "-client".to_string();
        };

        let name = format!("{}{client}{id}.log", session.name);
        let file = tmp.join(name);
        if !file.exists() {
            return file;
        }

        i += 1;
    }
}

fn next_available_session(sessions: &Path) -> Session {
    let mut i = 0;
    loop {
        for session in SESSION_NAMES {
            let number = if i == 0 {
                String::new()
            } else {
                format!("-{i}")
            };
            let session_name = format!("{session}{number}");
            let session = Session::new(sessions, &session_name);
            if !session.socket.exists() {
                return session;
            }
        }

        i += 1;
    }
}

fn start_server_process(cli: &Cli, session: &Session, server_opts: ServerOptions) {
    log::info!("Start server process..");
    let mut opts = vec!["--server-only", "--session", &session.name];
    if cli.debug {
        opts.push("--debug");
    }
    let config_dir = cli.config_dir.as_ref().map(|dir| dir.to_string_lossy());
    if let Some(ref dir) = config_dir {
        opts.push("--config-dir");
        opts.push(dir);
    }

    let working_dir = server_opts
        .working_dir
        .as_ref()
        .map(|dir| dir.to_string_lossy());
    if let Some(ref dir) = working_dir {
        opts.push("--working-dir");
        opts.push(dir);
    }

    let _ = std::process::Command::new("sane")
        .args(&opts)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .spawn();

    while !session.socket.exists() {
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn start_server(opts: ServerOptions) {
    sanedit_editor::run(opts);
}

/// Returns options back if could not connect to socket
fn connect_to_socket(socket: &Path, opts: ClientOptions) -> Option<ClientOptions> {
    log::info!("Connecting to existing socket..");
    // if socket already exists try to connect
    match UnixDomainSocketClient::connect(socket) {
        Ok(socket) => {
            socket.run(opts);
            None
        }
        Err(e) => {
            let _ = std::fs::remove_file(socket);
            if matches!(e.kind(), std::io::ErrorKind::ConnectionRefused) {
                Some(opts)
            } else {
                println!("Invalid session: {e}");
                None
            }
        }
    }
}
