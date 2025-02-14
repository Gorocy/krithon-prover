use std::net::SocketAddr;

use clap::Parser;
use communication::MessageType;
use http_body_util::Empty;
use hyper::{
    body::Bytes,
    header::{HeaderName, HeaderValue},
    Request, Uri,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use url::Url;
use utils::prover;

mod ast;
mod communication;
mod errors;
mod request;
mod response;
mod utils;

#[derive(Parser, Debug, Deserialize, Serialize, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    server_uri: Url,

    #[arg(short, long, default_value = "127.0.0.1:8079")]
    verifier_address: SocketAddr,

    #[arg(short = 'H', long)]
    headers: Vec<String>,

    #[arg(long, default_value = "4096")]
    max_sent_data: usize,

    #[arg(long, default_value = "16384")]
    max_recv_data: usize,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let mut stdout: tokio::io::Stdout = tokio::io::stdout();
    let mut stdin: tokio::io::Stdin = tokio::io::stdin();

    loop {
        let message_string = communication::read_message(&mut stdin).await;
        let test_message = serde_json::json!({
            "message": "Message received",
        });
        communication::send_response(test_message, MessageType::Message, &mut stdout).await;
        match message_string {
            Ok(message) => {
                let args: Args = match serde_json::from_str(&message) {
                    Ok(args) => args,
                    Err(e) => {
                        communication::logging_message(&mut stdout, &format!("Failed to parse arguments: {}", e)).await;
                        continue;
                    }
                };

                let host = match args.server_uri.host() {
                    Some(host) => host.to_owned(),
                    None => {
                        communication::logging_message(&mut stdout, "Server URI does not have a host").await;
                        continue;
                    }
                };

                let mut request = match Request::builder()
                    .method("GET")
                    .uri(args.server_uri.as_str())
                    .header("connection", "close")
                    .header("host", host.to_string())
                    .body(Empty::<Bytes>::new()) 
                {
                    Ok(req) => req,
                    Err(e) => {
                        communication::logging_message(&mut stdout, &format!("Failed to build request: {}", e)).await;
                        continue;
                    }
                };

                let request_headers = request.headers_mut();

                for header in args.headers {
                    // Split headers in the format "Key: Value"
                    if let Some((key, value)) = header.split_once(':') {
                        let key = match key.trim().parse::<HeaderName>() {
                            Ok(k) => k,
                            Err(e) => {
                                communication::logging_message(&mut stdout, &format!("Invalid header name '{}': {}", key, e)).await;
                                continue;
                            }
                        };
                        let value = match value.trim().parse::<HeaderValue>() {
                            Ok(v) => v,
                            Err(e) => {
                                communication::logging_message(&mut stdout, &format!("Invalid header value '{}': {}", value, e)).await;
                                continue;
                            }
                        };
                        request_headers.insert(key, value);
                    } else {
                        communication::logging_message(&mut stdout, &format!("Header '{}' is not in 'Key: Value' format", header)).await;
                        continue;
                    }
                }
                communication::logging_message(&mut stdout, "Request headers done").await;

                let socket = match TcpStream::connect(args.verifier_address).await {
                    Ok(s) => s,
                    Err(e) => {
                        communication::logging_message(&mut stdout, &format!("Failed to connect to verifier: {}", e)).await;
                        continue;
                    }
                };

                communication::logging_message(&mut stdout, "Connecting to verifier").await;

                if let Err(e) = prover(socket, request, args.max_sent_data, args.max_recv_data, &mut stdout).await {
                    communication::logging_message(&mut stdout, &format!("Prover encountered an error: {}", e)).await;
                    continue;
                }

                communication::logging_message(&mut stdout, "Prover done successfully").await;

            }
            Err(e) => {
                communication::send_error_response(&e.to_string(), &mut stdout).await;
                continue;
            }
        };
    }

    Ok(())
}
