use sanedit_utils::either::Either;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

use crate::error::LSPError;

pub(crate) const CONTENT_LENGTH: &str = "Content-Length";
pub(crate) const SEP: &str = "\r\n";

#[derive(Debug, Serialize)]
pub struct JsonRequest {
    jsonrpc: String,
    id: u32,
    method: String,
    params: Value,
}

impl JsonRequest {
    pub fn new<T>(method: &str, params: &T, id: u32) -> JsonRequest
    where
        T: ?Sized + Serialize,
    {
        let params = serde_json::to_value(params).unwrap();

        JsonRequest {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        }
    }

    pub async fn write_to<W: AsyncWriteExt + Unpin>(&self, stdin: &mut W) -> Result<(), LSPError> {
        let json = serde_json::to_string(&self)?;

        let clen = format!("{CONTENT_LENGTH}: {}{SEP}", json.len());
        stdin.write_all(clen.as_bytes()).await?;
        stdin.write_all(SEP.as_bytes()).await?;
        stdin.write_all(json.as_bytes()).await?;

        Ok(())
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JsonResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<lsp_types::LSPAny>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JsonResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: u32,
    pub result: Option<lsp_types::LSPAny>,
    pub error: Option<JsonResponseError>,
}

pub async fn read_from<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
) -> Result<Either<JsonResponse, JsonNotification>, LSPError> {
    let mut content_length = 0;
    let mut buf = vec![];

    // Read headers
    loop {
        buf.clear();

        let read = reader.read_until(b'\n', &mut buf).await?;
        if read == 0 {
            return Err(tokio::io::Error::from(tokio::io::ErrorKind::UnexpectedEof).into());
        }

        if buf.starts_with(CONTENT_LENGTH.as_bytes()) {
            let start = CONTENT_LENGTH.len() + ": ".len();
            let end = buf.len() - SEP.len();
            let clen = unsafe { std::str::from_utf8_unchecked(&buf[start..end]) };
            content_length = clen
                .parse()
                .map_err(|_| tokio::io::Error::from(tokio::io::ErrorKind::InvalidData))?;
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
        // {
        //     let content = unsafe { std::str::from_utf8_unchecked(&buf[..content_length]) };
        //     log::info!("READ: {content:?}");
        // }
        if let Ok(response) = serde_json::from_slice::<JsonResponse>(&buf[..content_length]) {
            return Ok(Either::Left(response));
        }

        if let Ok(notif) = serde_json::from_slice::<JsonNotification>(&buf[..content_length]) {
            return Ok(Either::Right(notif));
        }
    }

    Err(tokio::io::Error::from(tokio::io::ErrorKind::UnexpectedEof).into())
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct JsonNotification {
    #[allow(dead_code)]
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) params: Value,
}

impl JsonNotification {
    pub fn new<T>(method: &str, params: &T) -> JsonNotification
    where
        T: ?Sized + Serialize,
    {
        let params = serde_json::to_value(params).unwrap();

        JsonNotification {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }

    pub async fn write_to<W: AsyncWriteExt + Unpin>(&self, stdin: &mut W) -> Result<(), LSPError> {
        let json = serde_json::to_string(&self)?;

        let clen = format!("{CONTENT_LENGTH}: {}{SEP}", json.len());
        stdin.write_all(clen.as_bytes()).await?;
        stdin.write_all(SEP.as_bytes()).await?;
        stdin.write_all(json.as_bytes()).await?;

        Ok(())
    }
}
