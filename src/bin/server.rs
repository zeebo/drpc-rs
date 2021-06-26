use drpc::server;
use drpc::stream;
use std::net::TcpListener;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let lis = TcpListener::bind("127.0.0.1:8080")?;
    server::run(lis, EchoMux {})?;
    Ok(())
}

struct EchoMux;

impl server::Mux for EchoMux {
    fn serve<'a>(&self, rpc: &[u8], mut st: stream::GenericStream<'a>) -> stream::Result<()> {
        println!("{:?}: {:?}", st.id(), String::from_utf8_lossy(rpc));

        let mut buf = Vec::new();

        loop {
            st.recv_into::<(), Vec<u8>>(&mut buf)?;
            st.send::<(), Vec<u8>>(&buf)?;
        }
    }
}
