use http_body_util::Empty;
use hyper::{body::Bytes, Request as HyperRequest, StatusCode};
use hyper_util::rt::TokioIo;
use pest::Parser;
use pest_derive::Parser;
use tlsn_common::config::ProtocolConfig;
use tlsn_core::transcript::Idx;
use tlsn_prover::{state::Prove, Prover, ProverConfig};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
// use tracing::instrument;

use crate::ast::Searchable;
use crate::response::{Response, ResponseParser, Rule as ResponseRule};
use crate::request::{Request, RequestParser, Rule as RequestRule};


// #[instrument(skip(socket))]
pub async fn prover<T: AsyncWrite + AsyncRead + Send + Unpin + 'static>(
    socket: T,
    request: HyperRequest<Empty<Bytes>>,
    max_sent_data: usize,
    max_recv_data: usize,
) {
    assert_eq!(request.uri().scheme().unwrap().as_str(), "https");
    let server_domain = request.uri().authority().unwrap().host();
    let server_port = request.uri().port_u16().unwrap_or(443);

    // Create prover and connect to verifier.
    //
    // Perform the setup phase with the verifier.
    let prover = Prover::new(
        ProverConfig::builder()
            .server_name(server_domain)
            .protocol_config(
                ProtocolConfig::builder()
                    .max_sent_data(max_sent_data)
                    .max_recv_data(max_recv_data)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
    )
    .setup(socket.compat())
    .await
    .unwrap();

    // Connect to TLS Server.
    let tls_client_socket = tokio::net::TcpStream::connect((server_domain, server_port))
        .await
        .unwrap();

    // Pass server connection into the prover.
    let (mpc_tls_connection, prover_fut) =
        prover.connect(tls_client_socket.compat()).await.unwrap();

    // Wrap the connection in a TokioIo compatibility layer to use it with hyper.
    let mpc_tls_connection = TokioIo::new(mpc_tls_connection.compat());

    // Spawn the Prover to run in the background.
    let prover_task = tokio::spawn(prover_fut);

    // MPC-TLS Handshake.
    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(mpc_tls_connection)
            .await
            .unwrap();

    // Spawn the connection to run in the background.
    tokio::spawn(connection);

    let response = request_sender.send_request(request).await.unwrap();

    assert!(response.status() == StatusCode::OK);

    // Create proof for the Verifier.
    let mut prover = prover_task.await.unwrap().unwrap().start_prove();

    let idx_sent = redact_and_reveal_sent_data(&mut prover);
    let idx_recv = redact_and_reveal_received_data(&mut prover);

    // Reveal parts of the transcript
    prover.prove_transcript(idx_sent, idx_recv).await.unwrap();

    // Finalize.
    prover.finalize().await.unwrap()
}

/// Redacts and reveals received data to the verifier.
fn redact_and_reveal_received_data(prover: &mut Prover<Prove>) -> Idx {
    let recv_transcript = prover.transcript().received();

    let recv_string = String::from_utf8(recv_transcript.to_vec()).unwrap();
    let parse = ResponseParser::parse(ResponseRule::response, &recv_string).unwrap();
    let response = Response::try_from(parse).unwrap();
    let ranges =
        response.get_all_ranges_for_keypaths( &["state", "comment", "currency", "amount", "recipient.account" , "recipient.username", "recipient.code", "beneficiary.account"],&[]);

    Idx::new(ranges)
}

/// Redacts and reveals sent data to the verifier.
fn redact_and_reveal_sent_data(prover: &mut Prover<Prove>) -> Idx {
    let sent_transcript = prover.transcript().sent();
    let sent_string = String::from_utf8(sent_transcript.to_vec()).unwrap();
    let parse = RequestParser::parse(RequestRule::request, &sent_string)
        .expect("Failed to parse request");
    let request = Request::try_from(parse)
        .expect("Failed to convert request");
    
    let ranges = request.get_all_ranges_for_keypaths(&[], &["host"]);

    Idx::new(ranges)
}
