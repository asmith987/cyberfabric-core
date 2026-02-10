use bytes::Bytes;
use futures::StreamExt;

use crate::body::BoxStream;
use crate::error::ClientError;

/// A parsed Server-Sent Event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// Optional event ID
    pub id: Option<String>,
    /// Optional event type
    pub event: Option<String>,
    /// Event data
    pub data: String,
    /// Optional retry interval in milliseconds
    pub retry: Option<u64>,
}

/// Stream of Server-Sent Events parsed from a byte stream
pub struct SseEventStream {
    inner: BoxStream<Result<Bytes, ClientError>>,
    buffer: Vec<u8>,
}

impl SseEventStream {
    /// Create a new SSE event stream from a byte stream
    pub fn new(stream: BoxStream<Result<Bytes, ClientError>>) -> Self {
        Self {
            inner: stream,
            buffer: Vec::new(),
        }
    }

    /// Read the next SSE event from the stream
    pub async fn next_event(&mut self) -> Result<Option<SseEvent>, ClientError> {
        loop {
            // Check if buffer contains a complete event
            if let Some(event) = self.parse_buffered_event()? {
                return Ok(Some(event));
            }

            // Read more data from the stream
            match self.inner.next().await {
                Some(Ok(chunk)) => {
                    self.buffer.extend_from_slice(&chunk);
                }
                Some(Err(e)) => return Err(e),
                None => {
                    // Stream ended - parse any remaining buffered data
                    if self.buffer.is_empty() {
                        return Ok(None);
                    } else {
                        // Try to parse what's left
                        return self.parse_buffered_event();
                    }
                }
            }
        }
    }

    /// Try to parse a complete event from the buffer
    fn parse_buffered_event(&mut self) -> Result<Option<SseEvent>, ClientError> {
        // Find double newline (event separator)
        let separator = if let Some(pos) = self.find_double_newline() {
            pos
        } else {
            return Ok(None);
        };

        // Extract event bytes and remove from buffer
        let event_bytes: Vec<u8> = self.buffer.drain(..separator + 2).collect();

        // Parse the event
        Self::parse_sse_event(&event_bytes)
    }

    /// Find position of double newline in buffer
    fn find_double_newline(&self) -> Option<usize> {
        for i in 0..self.buffer.len().saturating_sub(1) {
            if self.buffer[i] == b'\n' && self.buffer[i + 1] == b'\n' {
                return Some(i);
            }
            if i < self.buffer.len().saturating_sub(3)
                && self.buffer[i] == b'\r'
                && self.buffer[i + 1] == b'\n'
                && self.buffer[i + 2] == b'\r'
                && self.buffer[i + 3] == b'\n'
            {
                return Some(i + 2);
            }
        }
        None
    }

    /// Parse a single SSE event from bytes
    fn parse_sse_event(data: &[u8]) -> Result<Option<SseEvent>, ClientError> {
        let text = std::str::from_utf8(data)
            .map_err(|e| ClientError::InvalidResponse(format!("Invalid UTF-8 in SSE: {}", e)))?;

        let mut id = None;
        let mut event = None;
        let mut data_lines = Vec::new();
        let mut retry = None;

        for line in text.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            // Parse field
            if let Some(colon_pos) = line.find(':') {
                let field = &line[..colon_pos];
                let value = line[colon_pos + 1..].trim_start();

                match field {
                    "id" => id = Some(value.to_string()),
                    "event" => event = Some(value.to_string()),
                    "data" => data_lines.push(value),
                    "retry" => {
                        if let Ok(retry_val) = value.parse::<u64>() {
                            retry = Some(retry_val);
                        }
                    }
                    _ => {} // Ignore unknown fields
                }
            } else if line.ends_with(':') {
                // Field with no value (e.g., "data:")
                let field = &line[..line.len() - 1];
                if field == "data" {
                    data_lines.push("");
                }
            }
        }

        // If no data was found, skip this event
        if data_lines.is_empty() {
            return Ok(None);
        }

        let data = data_lines.join("\n");

        Ok(Some(SseEvent {
            id,
            event,
            data,
            retry,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_parse_simple_event() {
        let data = b"data: hello world\n\n";
        let event = SseEventStream::parse_sse_event(data).unwrap().unwrap();
        assert_eq!(event.data, "hello world");
        assert_eq!(event.id, None);
        assert_eq!(event.event, None);
    }

    #[tokio::test]
    async fn test_parse_event_with_id() {
        let data = b"id: 123\nevent: message\ndata: hello\n\n";
        let event = SseEventStream::parse_sse_event(data).unwrap().unwrap();
        assert_eq!(event.data, "hello");
        assert_eq!(event.id, Some("123".to_string()));
        assert_eq!(event.event, Some("message".to_string()));
    }

    #[tokio::test]
    async fn test_parse_multiline_data() {
        let data = b"data: line 1\ndata: line 2\ndata: line 3\n\n";
        let event = SseEventStream::parse_sse_event(data).unwrap().unwrap();
        assert_eq!(event.data, "line 1\nline 2\nline 3");
    }

    #[tokio::test]
    async fn test_sse_stream() {
        let data = vec![
            Ok(Bytes::from("data: event 1\n\n")),
            Ok(Bytes::from("data: event 2\n\n")),
        ];
        let stream = Box::pin(stream::iter(data));
        let mut sse = SseEventStream::new(stream);

        let event1 = sse.next_event().await.unwrap().unwrap();
        assert_eq!(event1.data, "event 1");

        let event2 = sse.next_event().await.unwrap().unwrap();
        assert_eq!(event2.data, "event 2");

        let event3 = sse.next_event().await.unwrap();
        assert!(event3.is_none());
    }
}
