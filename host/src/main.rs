use std::{
    io::Write,
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::channel,
};

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use local_ip_address::local_ip;
use pnet::datalink;
use ringbuf::HeapRb;
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

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
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

    println!("Using input device: \"{}\"", input_device.name()?);

    // We'll try to keep the same configuration between streams to keep it simple.
    let config: cpal::StreamConfig = input_device.default_input_config()?.into();

    // Create a delay in case the input and output devices aren't synced
    let latency_frames = (opt.latency / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

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

        let ring = HeapRb::<f32>::new(latency_samples * 2);
        let (mut producer, mut consumer) = ring.split();

        for _ in 0..latency_samples {
            producer.push(0.0).unwrap();
        }

        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut output_fell_behind = false;
            for &sample in data {
                if producer.push(sample).is_err() {
                    output_fell_behind = true;
                }
            }
            if output_fell_behind {
                eprintln!("output stream fell behind: try increasing latency");
            }
        };

        // Build streams.
        println!(
            "Attempting to build input stream with f32 samples and `{:?}`",
            config
        );
        let input_stream = input_device.build_input_stream(&config, input_data_fn, err_fn, None)?;
        println!("Successfully built stream");

        // Play the streams.
        println!(
            "Starting the input streams with `{}` milliseconds of latency.",
            opt.latency
        );
        input_stream.play()?;

        std::thread::spawn(move || loop {
            if let Some(num) = consumer.pop() {
                if let Err(e) = client.write(&num.to_be_bytes()) {
                    eprintln!("{e}");
                }
            }
        });

        std::thread::sleep(std::time::Duration::from_secs(60));
    }

    Ok(())
}
