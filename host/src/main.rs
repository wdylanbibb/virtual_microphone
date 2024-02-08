use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn handle_client(mut stream: TcpStream) -> io::Result<File> {
    let buf: &mut [u8; 100] = &mut [0; 100];
    let mut file = File::create("foo")?;
    loop {
        let result = stream.read(buf);
        match result {
            Ok(len) => {
                let str = String::from_utf8(buf[0..len].to_vec());
                file.write_all(&buf[0..len])?;

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
    println!("Hello, world!");

    thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:34234").unwrap();

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
    });

    let mut client = TcpStream::connect("127.0.0.1:34234").unwrap();
    match client.write(b"I'm a teapot!") {
        Ok(len) => println!("wrote {} bytes", len),
        Err(e) => println!("error parsing header: {:?}", e),
    }
    thread::sleep(Duration::new(1, 0));
}
