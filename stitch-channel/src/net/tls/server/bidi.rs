use async_channel::{Receiver, Sender};
use async_std::io::*;
use async_std::net::*;
use async_std::task;
use async_tls::{server::TlsStream, TlsAcceptor};
use futures_util::io::{AsyncReadExt, ReadHalf, WriteHalf};

use crate::tls::server::{read, write};

pub struct BiDirectionalTlsServerChannel<T>
where
    T: Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    reader: read::ReadOnlyTlsServerChannel<T>,
    writer: write::WriteOnlyTlsServerChannel<T>,
}

impl<T> BiDirectionalTlsServerChannel<T>
where
    T: 'static + Send + Sync + serde::ser::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    pub fn unbounded(stream: TcpStream, acceptor: TlsAcceptor) -> Result<Self> {
        Self::from_parts(stream, acceptor, None, None)
    }

    pub fn bounded(
        stream: TcpStream,
        acceptor: TlsAcceptor,
        outgoing_bound: Option<usize>,
        incoming_bound: Option<usize>,
    ) -> Result<Self> {
        Self::from_parts(stream, acceptor, outgoing_bound, incoming_bound)
    }

    pub fn from_parts(
        stream: TcpStream,
        acceptor: TlsAcceptor,
        outgoing_bound: Option<usize>,
        incoming_bound: Option<usize>,
    ) -> Result<Self> {
        stream.set_nodelay(true)?;

        let encrypted_stream = task::block_on(acceptor.accept(stream))?;
        let streams = encrypted_stream.split();

        Self::from_raw_parts(
            streams,
            crate::channel_factory(outgoing_bound),
            crate::channel_factory(incoming_bound),
        )
    }

    pub(crate) fn from_raw_parts(
        (read_stream, write_stream): (
            ReadHalf<TlsStream<TcpStream>>,
            WriteHalf<TlsStream<TcpStream>>,
        ),
        outgoing_chan: (Sender<T>, Receiver<T>),
        incoming_chan: (Sender<T>, Receiver<T>),
    ) -> Result<Self> {
        Ok(BiDirectionalTlsServerChannel {
            reader: read::ReadOnlyTlsServerChannel::from_raw_parts(read_stream, incoming_chan)?,
            writer: write::WriteOnlyTlsServerChannel::from_raw_parts(write_stream, outgoing_chan)?,
        })
    }

    pub fn channel(&self) -> (Sender<T>, Receiver<T>) {
        (self.writer.tx_chan.0.clone(), self.reader.rx_chan.1.clone())
    }

    fn tls_channel(&self) -> (&Sender<T>, &Receiver<T>) {
        (&self.reader.rx_chan.0, &self.writer.tx_chan.1)
    }

    pub fn read_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.reader.task
    }

    pub fn write_task(&self) -> &task::JoinHandle<anyhow::Result<()>> {
        &self.writer.task
    }

    pub fn close(self) {
        self.reader.close();
        self.writer.close();
    }
}