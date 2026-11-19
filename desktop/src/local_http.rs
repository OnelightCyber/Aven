//! What Aven's two local endpoints share: the Claude Code bridge and the MCP
//! server. Both listen on the loopback interface, which every account and
//! process on the computer can reach, so every path starts with a secret
//! kept in the Credential Manager for this Windows account, compared in
//! constant time. A request is turned away on its head when it carries an
//! `Origin` (any web page), another `Host`, or no secret, before its body is
//! read; a body must be JSON, which a page cannot send across origins
//! without a preflight that is never answered.

use std::time::Duration;

use ring::rand::{SecureRandom, SystemRandom};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The most a request's line and headers may weigh.
const MAX_HEAD: usize = 64 * 1024;
/// How long a listener rests when the system refuses it a connection, out
/// of sockets or handles, rather than asking again at once.
pub(crate) const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

/// The secret every request to a local endpoint must carry, made on first
/// use and kept under `target` by the Credential Manager, for this Windows
/// account only.
pub(crate) fn secret(target: &str) -> Option<String> {
    let _turn = aven_credentials::turn().ok()?;
    let bytes = match aven_credentials::read(target) {
        Ok(bytes) if bytes.len() == 32 => bytes,
        Ok(_) | Err(aven_credentials::VaultError::Missing) => {
            let mut fresh = zeroize::Zeroizing::new(vec![0u8; 32]);
            SystemRandom::new().fill(&mut fresh).ok()?;
            aven_credentials::write(target, &fresh).ok()?;
            fresh
        }
        Err(_) => return None,
    };
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The path after the secret, when the request carries it. Compared in
/// constant time, so the answer's delay says nothing about the secret.
pub(crate) fn admitted<'a>(path: &'a str, secret: &str) -> Option<&'a str> {
    let rest = path.strip_prefix('/')?;
    let (first, rest) = rest.split_at(rest.find('/')?);
    same(first.as_bytes(), secret.as_bytes()).then_some(rest)
}

/// Whether two byte strings are equal, taking the same time whatever the
/// first byte that differs. The secret's length is no secret.
fn same(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |differing, (a, b)| differing | (a ^ b))
            == 0
}

pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) body: Vec<u8>,
}

/// Reads one request, refusing anything a web page could have sent. A
/// caller without the secret is turned away on its head, before its body is
/// read: a process that does not know it cannot make the browser hold what
/// it sends.
pub(crate) async fn read_request(
    stream: &mut TcpStream,
    port: u16,
    secret: &str,
    max_body: usize,
) -> Result<Request, u16> {
    let mut buffer = Vec::with_capacity(8192);
    let head_end = loop {
        if let Some(end) = find(&buffer, b"\r\n\r\n") {
            break end;
        }
        if buffer.len() > MAX_HEAD {
            return Err(431);
        }
        let mut chunk = [0u8; 8192];
        let read = stream.read(&mut chunk).await.map_err(|_| 400u16)?;
        if read == 0 {
            return Err(400);
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = std::str::from_utf8(&buffer[..head_end]).map_err(|_| 400u16)?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split(' ');
    let method = first.next().unwrap_or_default().to_owned();
    let path = first.next().unwrap_or_default().to_owned();
    let mut length = 0usize;
    let mut host_ok = false;
    let mut json_body = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(400);
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            // Any page's request carries its origin; the browser's own do not.
            "origin" => return Err(403),
            "host" => {
                host_ok =
                    value == format!("127.0.0.1:{port}") || value == format!("localhost:{port}");
            }
            "content-length" => length = value.parse().map_err(|_| 400u16)?,
            "content-type" => {
                json_body = value.to_ascii_lowercase().starts_with("application/json")
            }
            "transfer-encoding" => return Err(501),
            _ => {}
        }
    }
    if !host_ok {
        return Err(421);
    }
    if admitted(&path, secret).is_none() {
        return Err(401);
    }
    if method == "POST" && !json_body {
        return Err(415);
    }
    if length > max_body {
        return Err(413);
    }
    let mut body = buffer[head_end + 4..].to_vec();
    while body.len() < length {
        let mut chunk = vec![0u8; (length - body.len()).min(64 * 1024)];
        let read = stream.read(&mut chunk).await.map_err(|_| 400u16)?;
        if read == 0 {
            return Err(400);
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    Ok(Request { method, path, body })
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(crate) async fn respond_json(
    stream: &mut TcpStream,
    status: u16,
    body: &Value,
) -> std::io::Result<()> {
    let text = body.to_string();
    let head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        reason(status),
        text.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(text.as_bytes()).await?;
    stream.shutdown().await
}

pub(crate) async fn respond_error(
    stream: &mut TcpStream,
    status: u16,
    message: &str,
) -> std::io::Result<()> {
    respond_json(
        stream,
        status,
        &json!({"error": {"message": message, "type": "invalid_request_error"}}),
    )
    .await
}

pub(crate) fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        421 => "Misdirected Request",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// A body limit no test reaches.
    const LARGE_BODY: usize = 16 * 1024 * 1024;

    /// What `read_request` makes of `head` sent by a local caller, with a
    /// body announced and `body` actually sent.
    async fn read_from(head: &str, body: &'static [u8], max_body: usize) -> Result<Vec<u8>, u16> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let head = head.replace("{port}", &port.to_string());
        let caller = tokio::spawn(async move {
            let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(body).await.unwrap();
            // Held open: a reader that waited for more would wait for good.
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let (mut stream, _) = listener.accept().await.unwrap();
        let read = tokio::time::timeout(
            Duration::from_secs(2),
            read_request(&mut stream, port, &"ab".repeat(32), max_body),
        )
        .await
        .expect("answered at once");
        caller.abort();
        read.map(|request| request.body)
    }

    #[tokio::test]
    async fn a_caller_without_the_secret_is_turned_away_before_its_body_is_read() {
        let stranger = format!(
            "POST /{}/mcp HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\nContent-Type: application/json\r\nContent-Length: 1000000\r\n\r\n",
            "cd".repeat(32)
        );
        assert_eq!(read_from(&stranger, b"", LARGE_BODY).await, Err(401));
        // The secret, and a body larger than the server takes: refused unread.
        let large = format!(
            "POST /{}/mcp HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\nContent-Type: application/json\r\nContent-Length: 2000\r\n\r\n",
            "ab".repeat(32)
        );
        assert_eq!(read_from(&large, b"", 1000).await, Err(413));
        // A page's request, whatever it carries.
        let page = format!(
            "POST /{}/mcp HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\nOrigin: https://example.com\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n",
            "ab".repeat(32)
        );
        assert_eq!(read_from(&page, b"{}", LARGE_BODY).await, Err(403));
        let own = format!(
            "POST /{}/mcp HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n",
            "ab".repeat(32)
        );
        assert_eq!(read_from(&own, b"{}", LARGE_BODY).await, Ok(b"{}".to_vec()));
    }

    #[test]
    fn only_a_path_that_starts_with_the_secret_is_served() {
        let secret = "ab".repeat(32);
        assert_eq!(
            admitted(&format!("/{secret}/v1/models"), &secret),
            Some("/v1/models")
        );
        for path in [
            "/v1/models".to_owned(),
            "/v1/chat/completions".to_owned(),
            format!("/{}/v1/models", "cd".repeat(32)),
            format!("/{}/v1/models", &secret[..63]),
            format!("{secret}/v1/models"),
            format!("/{secret}"),
            String::new(),
        ] {
            assert_eq!(admitted(&path, &secret), None, "{path}");
        }
    }
}
