use std::{
    collections::{HashMap, VecDeque},
    io,
    net::TcpListener,
    os::fd::{AsRawFd, RawFd},
    ptr,
};

use io_uring::{opcode, types};
use slab::Slab;

const MAX_MSG_SIZE: usize = 4096;
const HEADER_SIZE: usize = 4;

#[derive(Clone, Debug)]
enum MessageState {
    Header,
    Message,
}

#[derive(Clone, Debug)]
enum Token {
    Accept,
    Read {
        fd: RawFd,
        buf_index: usize,
        state: MessageState,
    },
    Poll {
        fd: RawFd,
        state: MessageState,
    },
}

fn run_command(command: &Vec<String>, state: &mut HashMap<String, String>) {
    match command[0].as_str() {
        "get" => {
            let res = state.get(command[1].as_str());
            println!("Command result: {:?}", res);
        }
        "set" => {
            let res = state.insert(command[1].clone(), command[2].clone());
            println!("Command result: {:?}", res);
        }
        "del" => {
            let res = state.remove(&command[1].clone());
            println!("Command result: {:?}", res);
        }
        _ => {}
    }
}

fn main() -> anyhow::Result<()> {
    let mut ring = io_uring::IoUring::new(256)?;
    let (submitter, mut sq, mut cq) = ring.split();

    let listener = TcpListener::bind(("0.0.0.0", 8080))?;

    let mut backlog = VecDeque::new();
    let mut bufpool = Vec::with_capacity(64);
    let mut buf_alloc = Slab::with_capacity(64);
    let mut token_alloc = Slab::with_capacity(64);
    let mut commands = HashMap::<RawFd, Vec<String>>::new();

    let mut storage = HashMap::new();

    let token = token_alloc.insert(Token::Accept);

    let accept_op = opcode::Accept::new(
        types::Fd(listener.as_raw_fd()),
        ptr::null_mut(),
        ptr::null_mut(),
    )
    .build()
    .user_data(token as _);

    unsafe {
        sq.push(&accept_op).expect("submission queue is full");
        sq.sync();
    }

    loop {
        match submitter.submit_and_wait(1) {
            Ok(_) => (),
            Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => (),
            Err(err) => return Err(err.into()),
        }
        cq.sync();

        // clean backlog
        loop {
            if sq.is_full() {
                match submitter.submit() {
                    Ok(_) => (),
                    Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => break,
                    Err(err) => return Err(err.into()),
                }
            }
            sq.sync();

            match backlog.pop_front() {
                Some(sqe) => unsafe {
                    let _ = sq.push(&sqe);
                },
                None => break,
            }
        }

        unsafe {
            sq.push(&accept_op).expect("submission queue is full");
        }

        for cqe in &mut cq {
            let res = cqe.result();
            let token_index = cqe.user_data() as usize;

            if res < 0 {
                eprintln!(
                    "token {:?} error: {:?}",
                    token_alloc.get(token_index),
                    io::Error::from_raw_os_error(-res)
                );

                continue;
            }

            let token = &mut token_alloc[token_index];
            match token.clone() {
                Token::Accept => {
                    println!("Accept");
                    let socket_fd = res;

                    let poll_token = token_alloc.insert(Token::Poll {
                        fd: socket_fd,
                        state: MessageState::Header,
                    });

                    let poll_op = opcode::PollAdd::new(types::Fd(socket_fd), libc::POLLIN as _)
                        .build()
                        .user_data(poll_token as _);

                    unsafe {
                        if sq.push(&poll_op).is_err() {
                            backlog.push_back(poll_op);
                        }
                    }
                }

                Token::Poll { fd, state } => {
                    println!("Poll");
                    let (buf_index, buf) = match bufpool.pop() {
                        Some(buf_index) => (buf_index, &mut buf_alloc[buf_index]),
                        None => {
                            let buf = vec![0u8; 4 + MAX_MSG_SIZE + 1].into_boxed_slice();
                            let buf_entry = buf_alloc.vacant_entry();
                            let buf_index = buf_entry.key();
                            (buf_index, buf_entry.insert(buf))
                        }
                    };

                    let size_to_read = match state {
                        MessageState::Header => HEADER_SIZE,

                        MessageState::Message => MAX_MSG_SIZE,
                    };

                    *token = Token::Read {
                        fd,
                        buf_index,
                        state,
                    };

                    let read_op =
                        opcode::Recv::new(types::Fd(fd), buf.as_mut_ptr(), size_to_read as _)
                            .build()
                            .user_data(token_index as _);

                    unsafe {
                        if sq.push(&read_op).is_err() {
                            backlog.push_back(read_op);
                        }
                    }
                }

                Token::Read {
                    fd,
                    buf_index,
                    state,
                } => {
                    if res == 0 {
                        // Execute the command and return the result
                        run_command(commands.get(&fd).unwrap(), &mut storage);

                        bufpool.push(buf_index);
                        token_alloc.remove(token_index);

                        println!("shutdown");

                        unsafe {
                            libc::close(fd);
                        }
                    } else {
                        let len = res as usize;
                        let buf = &mut buf_alloc[buf_index];

                        match state {
                            MessageState::Header => {
                                let mut msg_size: [u8; HEADER_SIZE] = [0; HEADER_SIZE];
                                msg_size.clone_from_slice(&buf[..len]);

                                let parsed_msg_size: usize =
                                    u32::from_be_bytes(msg_size).try_into()?;

                                if parsed_msg_size == 0 {
                                    bufpool.push(buf_index);
                                    token_alloc.remove(token_index);

                                    println!("shutdown");

                                    unsafe {
                                        libc::close(fd);
                                    }
                                } else {
                                    *token = Token::Read {
                                        fd,
                                        buf_index,
                                        state: MessageState::Message,
                                    };

                                    let read_op = opcode::Recv::new(
                                        types::Fd(fd),
                                        buf.as_mut_ptr(),
                                        parsed_msg_size as _,
                                    )
                                    .build()
                                    .user_data(token_index as _);
                                    unsafe {
                                        if sq.push(&read_op).is_err() {
                                            backlog.push_back(read_op);
                                        }
                                    }
                                }
                            }
                            MessageState::Message => {
                                println!("Received message: {:?}", buf[..len].to_vec());
                                let commands_vec = match commands.get_mut(&fd) {
                                    Some(c) => c,
                                    None => {
                                        let v = Vec::new();
                                        commands.insert(fd, v);

                                        commands.get_mut(&fd).unwrap()
                                    }
                                };

                                commands_vec
                                    .push(std::str::from_utf8(&buf[..len]).unwrap().to_string());

                                bufpool.push(buf_index);

                                *token = Token::Poll {
                                    fd,
                                    state: MessageState::Header,
                                };
                                let poll_op =
                                    opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
                                        .build()
                                        .user_data(token_index as _);

                                unsafe {
                                    if sq.push(&poll_op).is_err() {
                                        backlog.push_back(poll_op);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
