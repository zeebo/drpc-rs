use drpc::conn;
use std::net::TcpStream;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let rpc = "/sesamestreet.CookieMonster/EatCookie".as_bytes();
    let mut socket = TcpStream::connect("localhost:8080")?;
    socket.set_nodelay(true)?;
    let mut conn = conn::Conn::new(&mut socket);

    let mut out: Vec<u8> = conn.invoke::<(), _, _>(rpc, &vec![8, 128, 10])?;
    println!("{:?}", &out);

    conn.invoke_into::<(), _, _>(rpc, &vec![8, 10], &mut out)?;
    println!("{:?}", &out);

    Ok(())
}
