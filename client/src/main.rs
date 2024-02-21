use std::io::{self, Read};
use std::{
    fs::File,
    net::{TcpListener, TcpStream},
    thread,
};

fn handle_client(mut stream: TcpStream) -> io::Result<File> {
    let buf: &mut [u8; 13] = &mut [0; 13];
    loop {
        let result = stream.read(buf);
        match result {
            Ok(len) => {
                let str = String::from_utf8(buf[0..len].to_vec());

                println!("wrote: {:?}", str);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                return Err(e);
            }
        }
    }
}
fn main() {
    let listener = TcpListener::bind("0.0.0.0:34234").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    // connection succeeded
                    handle_client(stream)
                });
            }
            Err(_) => { /* connection failed */ }
        }
    }
}
