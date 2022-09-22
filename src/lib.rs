use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

pub mod conn;
pub mod enc;
pub mod server;
pub mod stream;
pub mod transport;
pub mod wire;

pub trait Wire: Unpin + Send + AsyncRead + AsyncWrite {}

impl<T> Wire for T where T: Unpin + Send + AsyncRead + AsyncWrite {}

#[async_trait]
pub trait Transport: Send {
    fn wire(&mut self) -> &mut dyn Wire;

    async fn read_packet_into(
        &mut self,
        buf: &mut Vec<u8>,
    ) -> transport::Result<(wire::id::ID, wire::packet::Kind)>;
    async fn write_frame(&mut self, fr: wire::frame::Frame<'_>) -> transport::Result<()>;
    async fn flush(&mut self) -> transport::Result<()>;
}

#[async_trait]
pub trait Conn: Send {
    fn transport(&mut self) -> &mut dyn Transport;

    async fn invoke_into<In, Out>(
        &mut self,
        rpc: &[u8],
        input: &In,
        out: &mut Out,
    ) -> stream::Result<()>
    where
        In: enc::Marshal,
        Out: enc::Unmarshal;

    async fn invoke<In, Out>(&mut self, rpc: &[u8], input: &In) -> stream::Result<Out>
    where
        In: enc::Marshal,
        Out: enc::Unmarshal + Default,
    {
        let mut out = Default::default();
        self.invoke_into(rpc, input, &mut out).await?;
        Ok(out)
    }

    async fn stream<'s, In, Out>(
        &'s mut self,
        rpc: &[u8],
    ) -> stream::Result<Box<dyn Stream<In, Out> + 's>>
    where
        In: enc::Marshal + 's,
        Out: enc::Unmarshal + 's;
}

#[async_trait]
pub trait StreamSend<In: enc::Marshal>: Send {
    async fn send(&mut self, input: &In) -> stream::Result<()>;
}

#[async_trait]
pub trait StreamRecv<Out: enc::Unmarshal>: Send {
    async fn recv_into(&mut self, out: &mut Out) -> stream::Result<()>;
}

#[async_trait]
pub trait Stream<In: enc::Marshal, Out: enc::Unmarshal>:
    StreamSend<In> + StreamRecv<Out> + Send
{
    fn transport(&mut self) -> &mut dyn Transport;

    async fn invoke(&mut self, rpc: &[u8]) -> stream::Result<()>;

    async fn close_send(&mut self) -> stream::Result<()>;
    async fn close(&mut self) -> stream::Result<()>;
    async fn error(&mut self, msg: &str, code: u64) -> stream::Result<()>;
}
