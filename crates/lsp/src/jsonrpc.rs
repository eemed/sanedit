use anyhow::{bail, Result};
use sanedit_utils::either::Either;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

pub(crate) const CONTENT_LENGTH: &str = "Content-Length";
pub(crate) const SEP: &str = "\r\n";

#[derive(Debug, Serialize)]
pub struct Request {
    jsonrpc: String,
    id: u32,
    method: String,
    params: Value,
}

impl Request {
    pub fn new<T>(method: &str, params: &T) -> Request
    where
        T: ?Sized + Serialize,
    {
        let params = serde_json::to_value(params).unwrap();

        Request {
            jsonrpc: "2.0".into(),
            id: 1,
            method: method.into(),
            params,
        }
    }

    pub async fn write_to<W: AsyncWriteExt + Unpin>(&self, stdin: &mut W) -> Result<()> {
        let json = serde_json::to_string(&self)?;

        let clen = format!("{CONTENT_LENGTH}: {}{SEP}", json.len());
        stdin.write(clen.as_bytes()).await?;
        stdin.write(SEP.as_bytes()).await?;
        stdin.write(json.as_bytes()).await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<lsp_types::LSPAny>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: u32,
    pub result: Option<lsp_types::LSPAny>,
    pub error: Option<ResponseError>,
}

pub async fn read_from<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
) -> Result<Either<Response, Notification>> {
    let mut content_length = 0;
    let mut buf = vec![];

    // Read headers
    loop {
        buf.clear();

        let read = reader.read_until(b'\n', &mut buf).await?;
        if read == 0 {
            bail!("EOF encountered");
        }

        if buf.starts_with(CONTENT_LENGTH.as_bytes()) {
            let start = CONTENT_LENGTH.len() + ": ".len();
            let end = buf.len() - SEP.len();
            let clen = unsafe { std::str::from_utf8_unchecked(&buf[start..end]) };
            content_length = clen.parse()?;
        }

        if buf == SEP.as_bytes() {
            break;
        }
    }

    // Read content
    if content_length != 0 {
        while buf.len() < content_length {
            buf.push(b'\0');
        }
        reader.read_exact(&mut buf[..content_length]).await?;
        {
            let content = unsafe { std::str::from_utf8_unchecked(&buf[..content_length]) };
            log::info!("READ: {content:?}");
        }
        if let Ok(response) = serde_json::from_slice::<Response>(&buf[..content_length]) {
            return Ok(Either::Left(response));
        }

        if let Ok(notif) = serde_json::from_slice::<Notification>(&buf[..content_length]) {
            return Ok(Either::Right(notif));
        }
    }

    bail!("EOF")
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Notification {
    jsonrpc: String,
    method: String,
    params: Value,
}

impl Notification {
    pub fn new<T>(method: &str, params: &T) -> Notification
    where
        T: ?Sized + Serialize,
    {
        let params = serde_json::to_value(params).unwrap();

        Notification {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }

    pub async fn write_to<W: AsyncWriteExt + Unpin>(&self, stdin: &mut W) -> Result<()> {
        let json = serde_json::to_string(&self)?;

        let clen = format!("{CONTENT_LENGTH}: {}{SEP}", json.len());
        stdin.write(clen.as_bytes()).await?;
        stdin.write(SEP.as_bytes()).await?;
        stdin.write(json.as_bytes()).await?;

        Ok(())
    }
}
