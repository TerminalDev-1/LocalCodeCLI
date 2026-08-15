//! Turns a `reqwest` response body into a stream of text lines. Used by both the Ollama
//! (NDJSON) and OpenAI-compatible (SSE) adapters.
//!
//! Splitting on the raw `\n` byte is always safe for UTF-8: `0x0A` never appears as part of
//! a multi-byte sequence, so a line boundary is always a valid char boundary too.

use futures::{Stream, StreamExt};
use reqwest::Response;
use std::pin::Pin;

pub fn stream_lines(response: Response) -> Pin<Box<dyn Stream<Item = Result<String, reqwest::Error>> + Send>> {
    let byte_stream = response.bytes_stream();

    Box::pin(futures::stream::unfold((byte_stream, Vec::<u8>::new(), false), |(mut stream, mut buffer, mut done)| async move {
        loop {
            if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=pos).collect();
                let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]).trim().to_string();
                if line.is_empty() {
                    continue;
                }
                return Some((Ok(line), (stream, buffer, done)));
            }

            if done {
                if !buffer.is_empty() {
                    let rest = String::from_utf8_lossy(&buffer).trim().to_string();
                    buffer.clear();
                    if !rest.is_empty() {
                        return Some((Ok(rest), (stream, buffer, done)));
                    }
                }
                return None;
            }

            match stream.next().await {
                Some(Ok(bytes)) => buffer.extend_from_slice(&bytes),
                Some(Err(e)) => return Some((Err(e), (stream, buffer, true))),
                None => done = true,
            }
        }
    }))
}
