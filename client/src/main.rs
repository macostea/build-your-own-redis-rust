use std::{io::Write, net::TcpStream};

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;

    let mut s = Vec::with_capacity(64);

    let msg = "del";
    let msg2 = "a";
    let msg3 = "b";

    let bytes = (msg.len() as u32).to_be_bytes();
    s.append(&mut bytes.into());
    s.append(&mut msg.into());

    s.append(&mut (msg2.len() as u32).to_be_bytes().into());
    s.append(&mut msg2.into());

    // s.append(&mut (msg3.len() as u32).to_be_bytes().into());
    // s.append(&mut msg2.into());

    stream.write(&s)?;

    Ok(())
}
