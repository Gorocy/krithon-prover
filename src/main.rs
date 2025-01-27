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

    // tracing_subscriber::fmt::fmt()
    //     .with_max_level(Level::INFO)
    //     .init();

    // // Parse arguments
    // let args = Args::parse();

    loop {
        let message_string = communication::read_message(&mut stdin).await;
        let test_message = serde_json::json!({
            "message": "Message received",
        });
        communication::send_response(test_message, MessageType::Message, &mut stdout).await;
        match message_string {
            Ok(message) => {
                let test_message = serde_json::json!({
                    "message": "Message received second",
                    "args": message.clone()
                });
                communication::send_response(test_message, MessageType::Message, &mut stdout).await;
                let args: Args = serde_json::from_str(&message).unwrap();
                let example_json = serde_json::json!({
                    "message": "Message received",
                    "args": args.clone()
                });
                communication::send_response(example_json, MessageType::Message, &mut stdout).await;

                let host = args.server_uri.host().unwrap().to_owned();
                let mut request = Request::builder()
                    .method("GET")
                    .uri(args.server_uri.as_str())
                    .header("connection", "close")
                    .header("host", host.to_string())
                    .body(Empty::<Bytes>::new())
                    .unwrap();
                let request_headers = request.headers_mut();
                for header in args.headers {
                    // Split headers in the format "Key: Value"
                    if let Some((key, value)) = header.split_once(':') {
                        let key = key
                            .trim()
                            .parse::<HeaderName>()
                            .map_err(|e| format!("Invalid header name: {}", e))
                            .unwrap();
                        let value = value
                            .trim()
                            .parse::<HeaderValue>()
                            .map_err(|e| format!("Invalid header value: {}", e))
                            .unwrap();
                        request_headers.insert(key, value);
                    } else {
                        panic!("Header must be in the format 'Key: Value'");
                    }
                }

                let socket = TcpStream::connect(args.verifier_address).await?;
                prover(socket, request, args.max_sent_data, args.max_recv_data).await;
                communication::send_response(serde_json::json!({
                    "status": "success"
                }), MessageType::Message, &mut stdout).await;
            }
            Err(e) => {
                communication::send_error_response(&e.to_string(), &mut stdout).await;
                continue;
            }
        };
    }

    Ok(())
}
