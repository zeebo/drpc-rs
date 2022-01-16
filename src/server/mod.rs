use crate::transport::Stream;
use crate::{stream, transport, wire::packet};

use async_trait::async_trait;

use tokio::net;
use tokio::task;

#[async_trait]
pub trait Mux: Clone {
    async fn serve<'a>(
        &self,
        rpc: &[u8],
        stream: &mut stream::GenericStream<'a>,
    ) -> stream::Result<()>;
}

#[async_trait]
pub trait Listener<T> {
    async fn accept(&self) -> stream::Result<T>;
}

#[async_trait]
impl Listener<net::TcpStream> for net::TcpListener {
    async fn accept(&self) -> stream::Result<net::TcpStream> {
        let socket = net::TcpListener::accept(self).await.map(|s| s.0)?;
        socket.set_nodelay(true)?;
        Ok(socket)
    }
}

pub async fn run<L, W, M>(lis: L, mux: M) -> stream::Result<()>
where
    L: Listener<W>,
    W: transport::Wire + Send + 'static,
    M: Mux + Send + Sync + 'static,
{
    loop {
        let wire = lis.accept().await?;
        let mux = mux.clone();
        task::spawn(async move {
            let _ = handle_transport::<W, M>(wire, mux).await;
        });
    }
}

pub async fn handle_transport<W, M>(mut wire: W, mux: M)
where
    W: transport::Wire,
    M: Mux,
{
    let mut tr = transport::Transport::new(&mut wire);
    let mut mbuf = Vec::new();
    let mut sbuf = Vec::new();

    loop {
        let (id, kind) = match tr.read_packet_into(&mut mbuf).await {
            Ok((id, kind)) => (id, kind),
            Err(_) => return,
        };
        if kind != packet::Kind::Invoke {
            continue;
        }

        let mut st = stream::GenericStream::new(id.stream, &mut tr, &mut sbuf);
        match mux.serve(&mbuf, &mut st).await {
            Ok(()) => (),
            Err(stream::Error::StateError(stream::State::EOF)) => (),
            Err(err) => {
                let msg = err.to_string();
                let _ = st.error(&*msg, 10).await;
                return;
            }
        }
    }
}
