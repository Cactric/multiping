use console::{Term, style};
use std::{io::Error, net::SocketAddr, process::exit};
use clap::Parser;
use std::net::IpAddr;
use std::time::Duration;
use rand::random;

use multiping::{ping_host, HostInfo};

mod icmp;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Which hosts (IP addresses or domain names) to ping
    hosts: Vec<String>,
    
    /// How often the hosts should be pinged
    #[arg(short = 'i', long, default_value_t = 1.0)]
    interval: f32,
    
    /// If specified, min, max and average latency stats are shown
    #[arg(short, long)]
    statistics: bool,
}

fn say_hello() -> Result<(),Error> {
    let term = Term::stdout();
    term.clear_screen()?;
    term.write_line(&style("hello!").cyan().to_string())?;
    Ok(())
}

fn main() {
    //let _ = say_hello();
    
    // Parse arguments
    let args = Arguments::parse();

    if args.hosts.len() < 1 {
        eprintln!("You need to specify hosts on the command line.\nExample: multiping 127.0.0.1");
        exit(1);
    }
    
    // Try pinging
    for h in args.hosts {
        let mut maybe_hinfo = HostInfo::new(&h);
        if let Ok(mut hinfo) = maybe_hinfo {
            match ping_host(&mut hinfo) {
                Ok(()) => println!("Pinging {} succeeded.", h),
                Err(e) => println!("Pinging {} failed: {}.", h, e),
            }
        } else {
            eprintln!("Failed to parse {}", h);
        }
    }
}
