use std::{
    io::Write,
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::channel,
};

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use local_ip_address::local_ip;
use pnet::datalink;
use threadpool::ThreadPool;

#[derive(Parser, Debug)]
#[command(version, about = "CPAL feedback example", long_about = None)]
struct Opt {
    /// The input device to use
    #[arg(short, long, value_name = "IN", default_value_t = String::from("default"))]
    input_device: String,

    /// The output audio device to use
    #[arg(short, long, value_name = "OUT", default_value_t = String::from("default"))]
    output_device: String,

    /// Specify the delay between input and output
    #[arg(short, long, value_name = "DELAY_MS", default_value_t = 150.0)]
    latency: f32,

    /// Use the JACK host
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
        ),
        feature = "jack"
    ))]
    #[arg(short, long)]
    #[allow(dead_code)]
    jack: bool,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    // Conditionally compile with jack if the feature is specified
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
        ),
        feature = "jack"
    ))]
    // Manuall check for flags. Can be passed through cargo with -- e.g.
    // cargo run --release --example beep --features jack -- --jack
    let host = if opt.jack {
        cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available"
            )).expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    let host = cpal::default_host();

    // Find devices.
    let input_device = if opt.input_device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == opt.input_device).unwrap_or(false))
    }
    .expect("failed to find input device");

    let output_device = if opt.output_device == "default" {
        host.default_output_device()
    } else {
        host.output_devices()?
            .find(|x| x.name().map(|y| y == opt.output_device).unwrap_or(false))
    }
    .expect("failed to find output device");

    println!("Using input device: \"{}\"", input_device.name()?);
    println!("Using output device: \"{}\"", output_device.name()?);

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

    Ok(())

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
