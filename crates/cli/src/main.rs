mod logging;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use argh::FromArgs;
use sanedit_server::{Address, ServerOptions};
use sanedit_terminal_client::{unix::UnixDomainSocketClient, ClientOptions};

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

/// command line options
#[derive(FromArgs)]
struct Cli {
    /// file to open
    #[argh(positional)]
    file: Option<PathBuf>,

    /// turn debugging information on
    #[argh(switch)]
    debug: bool,

    /// set configuration directory
    #[argh(option)]
    config_dir: Option<PathBuf>,

    /// set working directory
    #[argh(option)]
    working_dir: Option<PathBuf>,

    /// connect to an existing session
    #[argh(option)]
    session: Option<String>,

    /// parent client, used to inherit options
    #[argh(option)]
    parent_client: Option<usize>,

    /// create server only, no UI
    #[argh(switch)]
    server_only: bool,

    /// list available sessions
    #[argh(switch)]
    list_sessions: bool,
}

fn main() {
    let cli: Cli = argh::from_env();

    if cli.list_sessions {
        list_sessions();
        return;
    }

    let socket = cli
        .session
        .as_ref()
        .map(|session| session_name_to_socket(&session))
        .unwrap_or_else(|| next_available_session_socket());

    let session = socket_to_session_name(&socket);
    let log_file = init_logging(&cli, &session);

    let connect_to_session = cli.session.is_some();
    let client_opts = ClientOptions {
        file: cli.file.clone(),
        parent_client: cli.parent_client,
        session: session.clone(),
    };
    let existing_session = connect_to_session || socket.try_exists().unwrap_or(false);
    let server_opts = ServerOptions {
        config_dir: cli.config_dir.clone(),
        working_dir: cli.working_dir.clone(),
        debug: cli.debug,
        addr: Address::UnixDomainSocket(socket.clone()),
    };

    if cli.server_only {
        start_server(server_opts);
        let _ = fs::remove_file(socket);
    } else if existing_session {
        connect_to_socket(&socket, client_opts);
    } else {
        start_server_process(&cli, &session, &socket);
        connect_to_socket(&socket, client_opts);
    }

    if let Some(log_file) = log_file {
        let _ = fs::remove_file(&log_file);
    }
}

fn list_sessions() {
    println!("Available sessions:");
    let mut paths = fs::read_dir("/tmp").unwrap();
    while let Some(Ok(path)) = paths.next() {
        let path = path.path();
        let name = path.to_string_lossy();
        if name.starts_with("/tmp/sanedit-") && name.ends_with(".sock") {
            let session = socket_to_session_name(&path);
            println!("{session}");
        }
    }
}

fn init_logging(cli: &Cli, session: &str) -> Option<PathBuf> {
    let tmp = sanedit_core::tmp_dir();
    if tmp.is_none() {
        eprintln!("TMP directory not accessible");
        return None;
    }
    let tmp = tmp.unwrap();
    let log_file = next_available_log_file(&tmp, cli, session);
    logging::init_panic();
    logging::init_logger(&log_file, cli.debug);

    Some(log_file)
}

fn next_available_log_file(tmp: &Path, cli: &Cli, session: &str) -> PathBuf {
    let mut i = 0;
    loop {
        let mut id = String::new();
        if i != 0 {
            id = format!("-{i}");
        }
        let mut client = String::new();
        if !cli.server_only {
            client = format!("-client");
        };

        let name = format!("sanedit-{session}{client}{id}.log",);
        let file = tmp.join(name);
        if !file.exists() {
            return file;
        }

        i += 1;
    }
}

fn next_available_session_socket() -> PathBuf {
    let mut i = 0;
    loop {
        for session in SESSION_NAMES {
            let number = if i == 0 {
                String::new()
            } else {
                format!("-{i}")
            };
            let session_name = format!("{session}{number}");
            let socket = session_name_to_socket(&session_name);
            if !socket.exists() {
                return socket;
            }
        }

        i += 1;
    }
}

fn session_name_to_socket(name: &str) -> PathBuf {
    let socket = format!("/tmp/sanedit-{name}.sock");
    PathBuf::from(socket)
}

fn socket_to_session_name(path: &Path) -> String {
    let name = path.to_string_lossy();
    name["/tmp/sanedit-".len()..name.len() - ".sock".len()].into()
}

fn start_server_process(cli: &Cli, session: &str, socket: &Path) {
    log::info!("Start server process..");
    let mut opts = vec!["sane", "--server-only", "--session", session];
    if cli.debug {
        opts.push("--debug");
    }
    let config_dir = cli.config_dir.as_ref().map(|dir| dir.to_string_lossy());
    if let Some(ref dir) = config_dir {
        opts.push("--config-dir");
        opts.push(&dir);
    }

    let working_dir = cli.working_dir.as_ref().map(|dir| dir.to_string_lossy());
    if let Some(ref dir) = working_dir {
        opts.push("--working-dir");
        opts.push(&dir);
    }

    let _ = std::process::Command::new("nohup")
        .args(&opts)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .spawn();

    while !socket.exists() {
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn start_server(opts: ServerOptions) {
    sanedit_editor::run(opts);
}

fn connect_to_socket(socket: &Path, opts: ClientOptions) {
    log::info!("Connecting to existing socket..");
    // if socket already exists try to connect
    match UnixDomainSocketClient::connect(&socket) {
        Ok(socket) => {
            socket.run(opts);
            return;
        }
        Err(e) => {
            println!("Invalid session: {e}");
            let _ = std::fs::remove_file(socket);
        }
    }
}
