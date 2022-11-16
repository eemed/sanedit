pub enum Address {
    UnixDomainSocket(),
    Tcp(),
}

#[derive(Debug)]
pub struct Client {
}

impl Client {
    pub fn connect(addr: Address) {
    }
}
