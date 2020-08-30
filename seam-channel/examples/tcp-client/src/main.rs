use log::*;
use seam_channel::net::{StitchClient, StitchNetClient};
use std::env;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Get ip address from cmd line args
    let args: Vec<String> = env::args().collect();
    let ip_address = match args.get(1) {
        Some(addr) => addr,
        None => {
            error!("Need to pass IP address to connect to as command line argument");
            panic!();
        }
    };

    // create a client connection to the server
    let conn = StitchNetClient::tcp_client(ip_address)?;

    // create a channel for String messages on the TCP connection
    let (sender, receiver) = conn.bounded::<String>(Some(100));

    // alert the connection that you are ready to read and write messages
    conn.ready()?;

    // send a message to the server
    let msg = String::from("Hello world");
    info!("Sending message: {}", msg);
    sender.send(msg).await?;

    // wait for the server to reply with an ack
    if let Ok(msg) = receiver.recv().await {
        info!("Received reply: {}", msg);
    }

    Ok(())
}