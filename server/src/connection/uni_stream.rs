use super::Connection;
use futures_util::StreamExt;
use quinn::{
    Connection as QuinnConnection, ConnectionError, IncomingUniStreams, RecvStream, VarInt,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::time;
use tuic_protocol::Command;

impl Connection {
    pub async fn listen_uni_streams(
        &self,
        mut uni_streams: IncomingUniStreams,
        expected_token_digest: [u8; 32],
    ) {
        while let Some(stream) = uni_streams.next().await {
            match stream {
                Ok(stream) => {
                    tokio::spawn(handle_uni_stream(
                        stream,
                        self.controller.clone(),
                        expected_token_digest,
                        self.is_authenticated.clone(),
                        self.create_time,
                    ));
                }
                Err(err) => {
                    match err {
                        ConnectionError::ConnectionClosed(_) | ConnectionError::TimedOut => {}
                        err => eprintln!("{err}"),
                    }
                    break;
                }
            }
        }
    }
}

async fn handle_uni_stream(
    mut stream: RecvStream,
    conn: QuinnConnection,
    expected_token_digest: [u8; 32],
    is_authenticated: Arc<AtomicBool>,
    create_time: Instant,
) {
    let cmd = match Command::read_from(&mut stream).await {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("{err}");
            conn.close(VarInt::MAX, b"Bad command");
            return;
        }
    };

    match cmd {
        Command::Authenticate { digest } => {
            if digest == expected_token_digest {
                is_authenticated.store(true, Ordering::Release);
            } else {
                eprintln!("Authentication failed");
                conn.close(VarInt::MAX, b"Authentication failed");
            }
        }
        cmd => {
            let mut interval = time::interval(Duration::from_millis(100));

            loop {
                if is_authenticated.load(Ordering::Acquire) {
                    match cmd {
                        Command::Authenticate { .. } => conn.close(VarInt::MAX, b"Bad command"),
                        Command::Connect { .. } => conn.close(VarInt::MAX, b"Bad command"),
                        Command::Bind { .. } => conn.close(VarInt::MAX, b"Bad command"),
                        Command::Packet {
                            assoc_id,
                            len,
                            addr,
                        } => todo!(),
                        Command::Dissociate { assoc_id } => todo!(),
                    }
                    break;
                } else if create_time.elapsed() > Duration::from_secs(3) {
                    eprintln!("Authentication timeout");
                    conn.close(VarInt::MAX, b"Authentication timeout");
                    break;
                } else {
                    interval.tick().await;
                }
            }
        }
    }
}