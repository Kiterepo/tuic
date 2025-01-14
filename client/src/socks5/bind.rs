use crate::relay::Request as RelayRequest;
use socks5_proto::{Address, Reply};
use socks5_server::{connection::bind::NeedFirstReply, Bind};
use std::io::{Error as IoError, ErrorKind};
use tokio::sync::mpsc::Sender;

pub async fn handle(
    conn: Bind<NeedFirstReply>,
    _req_tx: Sender<RelayRequest>,
    target_addr: Address,
) -> Result<(), IoError> {
    log::info!("[socks5] [{}] [bind] [{target_addr}]", conn.peer_addr()?);

    let mut conn = conn
        .reply(Reply::CommandNotSupported, Address::unspecified())
        .await?;

    let _ = conn.shutdown().await;

    Err(IoError::new(
        ErrorKind::Unsupported,
        "BIND command is not supported",
    ))
}
