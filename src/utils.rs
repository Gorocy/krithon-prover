use http_body_util::Empty;
use hyper::{body::Bytes, Request as HyperRequest, StatusCode};
use hyper_util::rt::TokioIo;
use pest::Parser;
use pest_derive::Parser;
use tlsn_common::config::ProtocolConfig;
use tlsn_core::transcript::Idx;
use tlsn_prover::{state::Prove, Prover, ProverConfig};
use tokio::io::{AsyncRead, AsyncWrite, Stdout};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
// use tracing::instrument;

use crate::ast::Searchable;
use crate::communication::logging_message;
use crate::errors::Errors;
use crate::request::{Request, RequestParser, Rule as RequestRule};
use crate::response::{Response, ResponseParser, Rule as ResponseRule};

// #[instrument(skip(socket))]
pub async fn prover<T: AsyncWrite + AsyncRead + Send + Unpin + 'static>(
    socket: T,
    request: HyperRequest<Empty<Bytes>>,
    max_sent_data: usize,
    max_recv_data: usize,
    stdout: &mut Stdout,
) -> Result<(), Errors> {
    if request.uri().scheme().map(|s| s.as_str()) != Some("https") {
        return Err(Errors::InvalidScheme);
    }
    let server_domain = match request.uri().authority().and_then(|auth| Some(auth.host())) {
        Some(domain) => domain.to_owned(),
        None => {
            return Err(Errors::MissingAuthority);
        }
    };
    let server_port = match request.uri().port_u16() {
        Some(port) => port,
        None => {
            logging_message(stdout, "No port found, using default port 443").await;
            443
        }
    };

    // Create prover configuration and connect to verifier.
    //
    // Perform the setup phase with the verifier.
    let prover_config = ProverConfig::builder()
        .server_name(server_domain.as_str())
        .protocol_config(
            ProtocolConfig::builder()
                .max_sent_data(max_sent_data)
                .max_recv_data(max_recv_data)
                .build()?,
        )
        .build()?;

    let prover = Prover::new(prover_config).setup(socket.compat()).await?;

    logging_message(stdout, "Prover setup done").await;

    // Connect to TLS Server.
    let tls_client_socket = tokio::net::TcpStream::connect((server_domain, server_port)).await?;

    logging_message(stdout, "Connecting to TLS Server").await;

    // Pass server connection into the prover.
    let (mpc_tls_connection, prover_fut) = prover.connect(tls_client_socket.compat()).await?;

    logging_message(stdout, "Prover connected to TLS Server").await;

    // Wrap the connection in a TokioIo compatibility layer to use it with hyper.
    let mpc_tls_connection = TokioIo::new(mpc_tls_connection.compat());

    logging_message(stdout, "Prover wrapped in TokioIo compatibility layer").await;
    // Spawn the Prover to run in the background.
    let prover_task = tokio::spawn(prover_fut);

    logging_message(stdout, "Prover spawned").await;

    // MPC-TLS Handshake.
    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(mpc_tls_connection).await?;

    logging_message(stdout, "MPC-TLS Handshake done").await;

    // Spawn the connection to run in the background.
    tokio::spawn(connection);

    logging_message(stdout, "Connection spawned").await;

    let response = request_sender.send_request(request).await?;

    logging_message(stdout, "Request sent").await;

    assert!(response.status() == StatusCode::OK);

    logging_message(stdout, "Response received is OK").await;
    // Create proof for the Verifier.
    let mut prover = prover_task.await??.start_prove();

    logging_message(stdout, "Prover started").await;

    let idx_sent = redact_and_reveal_sent_data(&mut prover).await?;
    logging_message(stdout, "Sent data redacted and revealed").await;

    let idx_recv = redact_and_reveal_received_data(&mut prover).await?;
    logging_message(stdout, "Received data redacted and revealed").await;

    // Reveal parts of the transcript
    prover.prove_transcript(idx_sent, idx_recv).await?;

    logging_message(stdout, "Transcript proof done").await;

    // Finalize.
    prover.finalize().await?;

    logging_message(stdout, "Prover finalized").await;

    Ok(())
}

/// Redacts and reveals received data to the verifier.
async fn redact_and_reveal_received_data(
    prover: &mut Prover<Prove>,
) -> Result<Idx, Errors> {
    let recv_transcript = prover.transcript().received();

    let recv_string = String::from_utf8(recv_transcript.to_vec())?;

    let parse = ResponseParser::parse(ResponseRule::response, &recv_string)?;

    let response = Response::try_from(parse).map_err(|e| Errors::StringError(e.to_string()))?;

    let ranges = response.get_all_ranges_for_keypaths(
        &[
            "state",
            "comment",
            "currency",
            "amount",
            "recipient.account",
            "recipient.username",
            "recipient.code",
            "beneficiary.account",
        ],
        &[],
    );

    Ok(Idx::new(ranges))
}

/// Redacts and reveals sent data to the verifier.
async fn redact_and_reveal_sent_data(
    prover: &mut Prover<Prove>,
) -> Result<Idx, Errors> {
    let sent_transcript = prover.transcript().sent();
    let sent_string =  String::from_utf8(sent_transcript.to_vec())?;

    let parse = RequestParser::parse(RequestRule::request, &sent_string)?;

    let request = Request::try_from(parse).map_err(|e| Errors::StringError(e.to_string()))?;

    let ranges = request.get_all_ranges_for_keypaths(&[], &["host"]);

    Ok(Idx::new(ranges))
}
