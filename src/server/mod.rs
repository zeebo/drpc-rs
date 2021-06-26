use crate::stream;
use crate::stream::Transport;
use crate::wire;
use crate::wire::packet;

use std::net;
use std::thread::spawn;

pub trait Mux {
    fn serve<'a>(&self, rpc: &[u8], stream: stream::GenericStream<'a>) -> stream::Result<()>;
}

impl<T: Mux> Mux for std::sync::Arc<T> {
    fn serve<'a>(&self, rpc: &[u8], stream: stream::GenericStream<'a>) -> stream::Result<()> {
        self.as_ref().serve(rpc, stream)
    }
}

pub trait Listener<T> {
    fn accept(&mut self) -> stream::Result<T>;
}

impl Listener<net::TcpStream> for net::TcpListener {
    fn accept(&mut self) -> stream::Result<net::TcpStream> {
        let socket = net::TcpListener::accept(self).map(|s| s.0)?;
        socket.set_nodelay(true)?;
        Ok(socket)
    }
}

pub fn run<L, T, M>(mut lis: L, mux: M) -> stream::Result<()>
where
    L: Listener<T>,
    T: wire::Transport + Send + 'static,
    M: Mux + Sync + Send + 'static,
{
    let mux = std::sync::Arc::new(mux);

    loop {
        let tr = lis.accept()?;
        let mux = mux.clone();
        spawn(move || {
            let _ = handle_transport(tr, mux);
        });
    }
}

pub fn handle_transport<T: wire::Transport, M: Mux>(mut tr: T, mux: M) -> stream::Result<()> {
    let mut tr = wire::transport::Transport::new(&mut tr);
    let mut mbuf = Vec::new();
    let mut sbuf = Vec::new();

    loop {
        let (id, kind) = tr.read_packet_into(&mut mbuf)?;
        if kind != packet::Kind::Invoke {
            continue;
        }
        let st = stream::GenericStream::new(id.stream, &mut tr, &mut sbuf);
        let _ = mux.serve(&mbuf, st);
    }
}
