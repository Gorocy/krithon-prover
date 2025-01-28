use std::borrow::Cow;

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Stdin, Stdout};

use crate::errors::{Errors, Result};

// use crate::errors::{Errors, Result};

#[derive(serde::Serialize)]
pub enum MessageType {
    Message,
    Logging,
}

// Helper function to send error messages as JSON
pub async fn send_error_response(error_msg: &str, stdout: &mut Stdout) {
    let json_error: Value = serde_json::json!({
        "type": "Error",
        "message": error_msg
    });
    let response_bytes = match serde_json::to_vec(&json_error) {
        Ok(bytes) => bytes,
        // TODO: how to handle this error?
        // If we cant send the error message, we should return an error
        Err(_err) => return,
    };
    let response_len = response_bytes.len() as u32;

    // Send the response length and error message
    if stdout.write_all(&response_len.to_le_bytes()).await.is_err() {
        // TODO: CONSIDER: what to do if we can't send the error message?
    }
    if stdout.write_all(&response_bytes).await.is_err() {
        // TODO: CONSIDER: what to do if we can't send the error message?
    }
    if stdout.flush().await.is_err() {
        // TODO: CONSIDER: what to do if we can't send the error message?
    };
}

// Helper function to send JSON responses
pub async fn send_response(json: Value, message_type: MessageType, stdout: &mut Stdout) {
    let json_message = serde_json::json!({
        "type": message_type,
        "message": json
    });

    let response_bytes = match serde_json::to_vec(&json_message) {
        Ok(bytes) => bytes,
        Err(e) => {
            send_error_response(&e.to_string(), stdout).await;
            return;
        }
    };
    let response_len = response_bytes.len() as u32;

    if let Err(e) = stdout.write_all(&response_len.to_le_bytes()).await {
        send_error_response(&e.to_string(), stdout).await;
        return;
    }
    if let Err(e) = stdout.write_all(&response_bytes).await {
        send_error_response(&e.to_string(), stdout).await;
        return;
    }
    if let Err(e) = stdout.flush().await {
        send_error_response(&e.to_string(), stdout).await;
        return;
    }
}

pub async fn read_message(stdin: &mut Stdin) -> Result<String> {
    // Read the length of the message (4 bytes, little-endian)
    let mut len_bytes = [0u8; 4];
    if stdin.read_exact(&mut len_bytes).await.is_err() {
        return Err(Errors::FailedToReadSizeFromExtension);
    }
    let len = u32::from_le_bytes(len_bytes) as usize;

    // Read the message
    let mut buffer = vec![0u8; len];
    if stdin.read_exact(&mut buffer).await.is_err() {
        return Err(Errors::FailedToReadMessageFromExtension);
    }
    let message: Cow<'_, str> = String::from_utf8_lossy(&buffer);
    let message_string = message.to_string();

    Ok(message_string)
}

pub async fn logging_message(stdout: &mut Stdout, message: &str) {
    let logging_message = serde_json::json!({
        "logging": message,
    });
    send_response(logging_message, MessageType::Logging, stdout).await;
}
