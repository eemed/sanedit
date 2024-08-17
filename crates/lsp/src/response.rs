#[derive(Debug, Clone)]
pub enum Response {
    Request(RequestResult),
    // Notification(),
}

#[derive(Debug, Clone)]
pub enum RequestResult {
    Hover {},
}
