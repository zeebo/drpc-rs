use drpc::conn;
use std::net::TcpStream;

fn main() -> drpc::Result<()> {
    let rpc = "/sesamestreet.CookieMonster/EatCookie";
    let mut socket = TcpStream::connect("localhost:8080")?;
    let mut conn = conn::Conn::new(&mut socket);

    let mut out: Vec<u8> = conn.invoke::<(), _, _>(rpc, &vec![8, 128, 10])?;
    println!("{:?}", &out);

    let mut stream = conn.stream::<(), _, _>(rpc)?;
    stream.send(&vec![8, 10])?;
    println!("{:?}", stream.recv()?);
    stream.close()?;

    conn.invoke_into::<(), _, _>(rpc, &vec![8, 10], &mut out)?;
    println!("{:?}", &out);

    Ok(())
}
