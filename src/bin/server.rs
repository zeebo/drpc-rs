use drpc::{server, stream};

use async_trait::async_trait;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let lis = TcpListener::bind("127.0.0.1:8080").await?;
    server::run(lis, EchoMux {}).await?;
    Ok(())
}

#[derive(Clone)]
struct EchoMux;

#[derive(Debug)]
struct Foo;

impl std::fmt::Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Foo {}

#[async_trait]
impl server::Mux for EchoMux {
    async fn serve<'a>(
        &self,
        rpc: &[u8],
        st: &mut stream::GenericStream<'a>,
    ) -> stream::Result<()> {
        println!("{:?}: {:?}", st.id(), String::from_utf8_lossy(rpc));

        let mut buf = Vec::new();
        // let mut st = st.fix::<(), Vec<u8>, Vec<u8>>();

        Err(stream::Error::AnyError(Box::new(Foo)))?;

        loop {
            st.recv_into::<(), _>(&mut buf).await?;
            st.send::<(), _>(&buf).await?;
        }
    }
}
