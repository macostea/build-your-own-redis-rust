use std::{net::TcpStream, io::{Write, Read}};

fn main() -> std::io::Result<()> {
  let mut stream = TcpStream::connect("127.0.0.1:8080")?;

  let mut s = [0; 64];

  stream.write(&[42])?;
  stream.read(&mut s)?;

  println!("Read from server: {:?}", s);
  
  Ok(())
}
