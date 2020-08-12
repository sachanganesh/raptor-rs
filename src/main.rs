use async_std::task;
use async_channel::{Receiver, Sender};
use uuid::Uuid;

use stitch_channel::{tcp::BiDirectionalTcpChannel as TcpChannel, tls::{BiDirectionalTlsClient as TlsChannel, async_tls::TlsConnector, rustls::ClientConfig, BiDirectionalTlsClient}};
use stitch_channel::tcp::TcpServer;
use std::time::Duration;

#[macro_use] extern crate log;

const MAX_MESSAGES: usize = 150;
const DOMAIN: &str = "localhost";
const IP_ADDR: &str = "localhost:5678";

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let echo_server = TcpServer::unbounded(IP_ADDR, handle_connections)?;

    task::block_on(task::sleep(Duration::from_secs(1)));

    let dist_chan = test_tcp();
    // let dist_chan = test_tls();

    let (sender, receiver): (Sender<String>, Receiver<String>) = dist_chan.channel();

    let read_task = task::spawn(async_read(receiver));
    let _write_task = task::spawn(async_write(sender));

    task::block_on(read_task);

    Ok(())
}

async fn handle_connections((sender, receiver): (Sender<String>, Receiver<String>)) {
    debug!("Starting echo loop");

    while let Ok(data) = receiver.recv().await {
        info!("Echoing: {}", data);
        sender.send(data).await.expect("it works");
    }
}

fn test_tcp() -> TcpChannel<String> {
    TcpChannel::unbounded(IP_ADDR).expect("it works")
}

fn test_tls() -> TlsChannel<String> {
    let file = std::fs::read("/home/svganesh/Documents/tools/echo-server/async-tls/tests/end.chain").unwrap();
    let mut pem = std::io::Cursor::new(file);

    let mut config = ClientConfig::new();
    config
        .root_store
        .add_pem_file(&mut pem)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid cert"))
        .expect("it works");

    let connector: TlsConnector = config.into();

    match TlsChannel::unbounded(IP_ADDR, DOMAIN, connector) {
        Ok(data) => data,
        Err(err) => {
            println!("{}", err);
            panic!(err);
        }
    }
}

async fn async_read(receiver: Receiver<String>) {
    for i in 0..MAX_MESSAGES + 1 {
        if let Ok(data) = receiver.recv().await {
            info!("Received #{}: {}", i, data);
        }
    }
}

async fn async_write(sender: Sender<String>) -> Result<(), anyhow::Error> {
    for i in 0..MAX_MESSAGES {
        let id = Uuid::new_v4();
        let msg = format!("Hello, {}", id);

        info!("Sending #{}: {}", i, msg);
        sender.send(msg).await?;
    }

    Ok(())
}
