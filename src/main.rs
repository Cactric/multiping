use console::{Term, style};
use std::io::Error;
use clap::Parser;
use std::net::IpAddr;
use std::time::Duration;
use rand::random;

use multiping::HostInfo;

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
    let _ = say_hello();
    
    // Parse arguments
    let args = Arguments::parse();
    
    // Try pinging
}
