use std::{net::TcpStream, io::{Write, Read}};

fn main() -> std::io::Result<()> {
  let mut stream = TcpStream::connect("127.0.0.1:8080")?;

  let mut s = [0; 64];

  let msg = "Hello world";
  let bytes = (msg.len() as u32).to_be_bytes();
  s[..4].clone_from_slice(&bytes);
  s[4..4 + msg.len()].clone_from_slice(msg.as_bytes());

  stream.write(&s)?;
  
  Ok(())
}
