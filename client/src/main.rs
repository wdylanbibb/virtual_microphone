use std::io::Read;
use std::net::TcpListener;

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;

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

    let output_device = if opt.output_device == "default" {
        host.default_output_device()
    } else {
        host.output_devices()?
            .find(|x| x.name().map(|y| y == opt.output_device).unwrap_or(false))
    }
    .expect("failed to find output device");

    println!("Using output device: \"{}\"", output_device.name()?);

    // We'll try to keep the same configuration between streams to keep it simple.
    let config: cpal::StreamConfig = input_device.default_input_config()?.into();

    // Create a delay in case the input and output devices aren't synced.
    let latency_frames = (opt.latency / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

    // Run for 3 seconds before closing.
    // println!("Playing for 3 seconds...");
    // std::thread::sleep(std::time::Duration::from_secs(3));
    // drop(input_stream);
    // drop(output_stream);
    // println!("Done!");

    let listener = TcpListener::bind("0.0.0.0:34234").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                // Connection successful
                // The buffer to share samples
                let ring = HeapRb::<f32>::new(latency_samples * 2);
                let (mut producer, mut consumer) = ring.split();

                // Fill the samples with 0.0 equal to the length of the delay.
                for _ in 0..latency_samples {
                    // The ring buffer has twice as much space as necessary to add latency here,
                    // so this should never fail
                    producer.push(0.0).unwrap();
                }

                let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut input_fell_behind = false;
                    for sample in data {
                        *sample = match consumer.pop() {
                            Some(s) => s,
                            None => {
                                input_fell_behind = true;
                                0.0
                            }
                        };
                    }
                    if input_fell_behind {
                        eprintln!("input stream fell behind: try increasing latency");
                    }
                };

                // Build streams.
                println!(
                    "Attempting to build stream with f32 sample and `{:?}`",
                    config
                );
                let output_stream =
                    output_device.build_output_stream(&config, output_data_fn, err_fn, None)?;
                println!("Successfully built streams");

                // Play the streams.
                println!(
                    "Starting the output stream with `{}` milliseconds of latency.",
                    opt.latency
                );
                output_stream.play()?;

                // handle_client(stream).unwrap();
                let buf: &mut [u8; 4] = &mut [0; 4];
                loop {
                    let result = stream.read(buf);
                    match result {
                        Ok(len) => {
                            if len > 0 {
                                let str = f32::from_be_bytes(*buf);

                                println!("wrote: {:?}", str);
                                if let Err(e) = producer.push(str) {
                                    eprintln!("{e}");
                                }
                            }
                        }
                        Err(e) => {
                            println!("error parsing header: {:?}", e);
                        }
                    }
                }
            }
            Err(_) => { /* connection failed */ }
        }
    }

    Ok(())
}
