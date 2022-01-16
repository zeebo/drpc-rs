use drpc::conn;

use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let rpc = "/sesamestreet.CookieMonster/EatCookie".as_bytes();
    let mut socket = TcpStream::connect("localhost:8080").await?;
    socket.set_nodelay(true)?;
    let mut conn = conn::Conn::new(&mut socket);

    let mut out = conn.invoke::<(), _, _>(rpc, &vec![8, 128, 10]).await?;
    println!("{:?}", &out);

    conn.invoke_into::<(), _, _>(rpc, &vec![8, 10], &mut out)
        .await?;
    println!("{:?}", &out);

    Ok(())
}
