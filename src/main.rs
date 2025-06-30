use console::{Term, style};
use std::io::Error;
use clap::Parser;
use std::net::IpAddr;
use std::time::Duration;
use rand::random;

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

struct HostInfo {
    // Times in this struct are in milliseconds? (unless I change them to a Duration)
    host: String,
    pings_sent: i32,
    latest_time: Option<i32>,
    sum_times: i64,
    min_time: i32,
    max_time: i32,
    successful: i32,
    last_error: i32, /* TODO: replace with errno or struct */
}

impl HostInfo {
    pub fn average(&self) -> f32 {
        self.sum_times as f32 / self.pings_sent as f32
    }
    // TODO: jitter? (std. deviation of times)
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
    let random = random();
    let target_ip = "127.0.0.1".parse().unwrap();
    let timeout = Duration::from_secs_f32(args.interval);
    ping::dgramsock::ping(target_ip, Some(timeout), Some(166), Some(3), Some(5), Some(&random)).unwrap();
}
