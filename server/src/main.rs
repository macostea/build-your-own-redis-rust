use std::{net::{TcpListener, TcpStream}, io::{Read, Write}, error::Error};

const MAX_MSG_SIZE: usize = 4096;
const HEADER_SIZE: usize = 4;

fn handle_client(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buf = [0; 4 + MAX_MSG_SIZE + 1];

    // Read header
    let header_size = stream.read(&mut buf[0 .. HEADER_SIZE])?;

    if header_size != HEADER_SIZE {
        return Err("Mismatch header size".into());
    }

    let mut msg_size: [u8; HEADER_SIZE] = [0; HEADER_SIZE];
    msg_size.clone_from_slice(&buf[.. header_size]);
    let parsed_msg_size: usize = u32::from_be_bytes(msg_size).try_into()?;

    let msg_size = stream.read(&mut buf[header_size .. header_size + parsed_msg_size])?;

    println!("Read message from client: {:?}", &buf[header_size .. header_size + msg_size]);

    Ok(())
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if handle_client(stream).is_err() {
                    println!("Failed to read from client");
                }
            }
            Err(e) => {
                println!("Connection failed {}", e);
            }
        }
    }
    Ok(())
}
