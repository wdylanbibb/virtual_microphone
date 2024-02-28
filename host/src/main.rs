use std::{
    io::Write,
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::channel,
};

use local_ip_address::local_ip;
use pnet::datalink;
use threadpool::ThreadPool;

fn main() {
    let local_ip = local_ip().unwrap();
    println!("{}", local_ip);
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

    let pool = ThreadPool::new(256);

    let (tx, rx) = channel();
    match local_ip {
        std::net::IpAddr::V4(ip) => {
            let ip: u32 = u32::from_be_bytes(ip.octets()) & (u32::MAX << prefix);
            for i in 0..2u32.pow(prefix.into()) {
                let tx = tx.clone();
                pool.execute(move || {
                    let test_ip = Ipv4Addr::from((ip + i).to_be_bytes());
                    if let Ok(c) = TcpStream::connect((test_ip, 34234)) {
                        tx.send(c)
                            .expect("channel will be there waiting for the pool");
                    };
                })
            }
        }
        std::net::IpAddr::V6(_) => (),
    }

    for mut client in rx.iter() {
        println!("{:?}", client);
        match client.write(b"I'm a teapot!") {
            Ok(len) => println!("wrote {} bytes", len),
            Err(e) => println!("error parsing header: {:?}", e),
        }
    }

    // let mut client = match TcpStream::connect("192.168.86.46:34234") {
    //     Ok(c) => c,
    //     Err(e) => {
    //         println!("Could not connect: {:?}", e);
    //         return;
    //     }
    // };
    // loop {
    //     match client.write(b"I'm a teapot!") {
    //         Ok(len) => println!("wrote {} bytes", len),
    //         Err(e) => println!("error parsing header: {:?}", e),
    //     }
    //     // thread::sleep(Duration::new(1, 0));
    // }
}
