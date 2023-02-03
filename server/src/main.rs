use std::{net::{TcpListener, TcpStream}, io::{Read, Write}, error::Error};

const MAX_MSG_SIZE: usize = 4096;
const HEADER_SIZE: usize = 4;

fn read_single_message(mut stream: &TcpStream, start: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = [0; 4 + MAX_MSG_SIZE + 1];

    // Read header
    let header_size = stream.read(&mut buf[start .. start + HEADER_SIZE])?;

    if header_size != HEADER_SIZE {
        return Err("Mismatch header size".into());
    }

    let mut msg_size: [u8; HEADER_SIZE] = [0; HEADER_SIZE];
    msg_size.clone_from_slice(&buf[start .. start + header_size]);
    let parsed_msg_size: usize = u32::from_be_bytes(msg_size).try_into()?;

    if parsed_msg_size == 0 {
        return Err("No more msg".into());
    }

    let msg_size = stream.read(&mut buf[start + header_size .. start + header_size + parsed_msg_size])?;

    let msg = buf[start + header_size .. start + header_size + msg_size].to_vec();

    Ok(msg)
}

fn handle_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut current_msg_start = 0;
    while let Ok(res) = read_single_message(&stream, current_msg_start) {
        println!("Read from client {:?}", res);
        current_msg_start += res.len();
    }

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
