use tokio::net::UnixStream;

pub(crate) async fn run_unix_domain_socket<P: AsRef<Path>>(path: P) {
    let conn = UnixStream::connect(path).await;
    let (read, write) = conn.split();
}

pub(crate) struct UnixDomainSocketClient {
    conn: UnixStream,
}
