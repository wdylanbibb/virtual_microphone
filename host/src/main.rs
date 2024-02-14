use std::io::prelude::*;
use std::net::TcpStream;

fn main() {
    let mut client = TcpStream::connect("127.0.0.1:34234").unwrap();
    loop {
        match client.write(b"I'm a teapot!") {
            Ok(len) => println!("wrote {} bytes", len),
            Err(e) => println!("error parsing header: {:?}", e),
        }
        // thread::sleep(Duration::new(1, 0));
    }
}
