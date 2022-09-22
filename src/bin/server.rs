use drpc::{enc, server, stream};

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

#[async_trait]
impl server::Mux for EchoMux {
    async fn serve<'a>(&self, rpc: &[u8], st: &mut stream::Stream<'a>) -> stream::Result<()> {
        println!("{:?}: {:?}", st.id(), String::from_utf8_lossy(rpc));

        rpc1(st).await?;
        rpc2(st).await?;

        println!("done");

        Ok(())
    }
}

struct Foo;

impl enc::Marshal for Foo {
    fn marshal(&self, _: &mut Vec<u8>) -> enc::Result<()> {
        Ok(())
    }
}

impl enc::Unmarshal for Foo {
    fn unmarshal(&mut self, _: &[u8]) -> enc::Result<()> {
        Ok(())
    }
}

async fn rpc1<S>(s: &mut S) -> stream::Result<()>
where
    S: drpc::Stream<Foo, Foo>,
{
    s.recv_into(&mut Foo {}).await?;
    s.send(&Foo {}).await?;
    Ok(())
}

async fn rpc2<S>(s: &mut S) -> stream::Result<()>
where
    S: drpc::Stream<Vec<u8>, Vec<u8>>,
{
    let mut i = vec![];
    s.recv_into(&mut i).await?;
    // s.send(&vec![1, 2, 3]).await?;
    Ok(())
}
