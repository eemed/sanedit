use anyhow::{bail, Result};
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

    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

impl Request {
    pub fn new<T>(method: &str, params: Option<&T>) -> Request
    where
        T: ?Sized + Serialize,
    {
        let params = params.map(serde_json::to_value).map(|p| p.unwrap());

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

#[derive(Debug, Deserialize)]
struct ResponseError {
    code: i32,
    message: String,
    data: Option<lsp_types::LSPAny>,
}

#[derive(Debug, Deserialize)]
pub struct Response {
    jsonrpc: String,
    id: u32,
    result: Option<lsp_types::LSPAny>,
    error: Option<ResponseError>,
}

impl Response {
    pub async fn read_from<R: AsyncBufReadExt + Unpin>(reader: &mut R) -> Result<Response> {
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
            // let content = unsafe { std::str::from_utf8_unchecked(&buf[..content_length]) };
            let response: Response = serde_json::from_slice(&buf[..content_length])?;
            return Ok(response);
        }

        bail!("EOF")
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Notification {
    jsonrpc: String,
    method: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

impl Notification {
    pub fn new<T>(method: &str, params: Option<&T>) -> Notification
    where
        T: ?Sized + Serialize,
    {
        let params = params.map(serde_json::to_value).map(|p| p.unwrap());

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
