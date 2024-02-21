use std::io::prelude::*;
use std::net::{Ipv4Addr, TcpStream};

use local_ip_address::local_ip;
use pnet::datalink;

fn main() {
    let local_ip = local_ip().unwrap();
    let prefix = 32 - {
        let mut prefix = None;
        for iface in datalink::interfaces() {
            for ip in iface.ips {
                match ip {
                    pnet::ipnetwork::IpNetwork::V4(i) => {
                        if i.ip() == local_ip {
                            prefix = Some(i.prefix());
                        }
                    }
                    pnet::ipnetwork::IpNetwork::V6(_) => {}
                }
            }
        }
        prefix
    }
    .unwrap();
    match local_ip {
        std::net::IpAddr::V4(ip) => {
            let ip: u32 = u32::from_be_bytes(ip.octets()) & (u32::MAX << prefix);
            for i in 0..2u32.pow(prefix.into()) {
                let test_ip_bits = (ip + i).to_be_bytes();
                let test_ip = Ipv4Addr::new(
                    test_ip_bits[0],
                    test_ip_bits[1],
                    test_ip_bits[2],
                    test_ip_bits[3],
                );
                let mut client = match TcpStream::connect((test_ip, 34234)) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
            }
        }
        std::net::IpAddr::V6(_) => (),
    }
    // let mut client = match TcpStream::connect("192.168.86.25:34234") {
    //     Ok(c) => c,
    //     Err(_) => return,
    // };
    // loop {
    //     match client.write(b"I'm a teapot!") {
    //         Ok(len) => println!("wrote {} bytes", len),
    //         Err(e) => println!("error parsing header: {:?}", e),
    //     }
    //     // thread::sleep(Duration::new(1, 0));
    // }
}
