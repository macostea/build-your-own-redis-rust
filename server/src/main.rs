use std::{net::{TcpListener, TcpStream}, io::{Read, Write}};

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut s: [u8; 64] = [0; 64];
    stream.read(&mut s)?;

    println!("Read from client: {:?}", s);

    stream.write(&s)?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;

    for stream in listener.incoming() {
        if handle_client(stream?).is_err() {
            println!("Failed to read from client");
        }
    }
    Ok(())
}
