use std::net::SocketAddr;
use std::io::{Error, Read, ErrorKind};
use std::time::SystemTime;
use std::net::ToSocketAddrs;
use std::net::SocketAddr::V6;
use console::style;
use socket2::{Domain, Protocol, Socket, Type};

use crate::icmp::*;

pub mod icmp;

#[derive(Clone, Debug)]
pub struct HostInfo {
    // Times in this struct are in microseconds? (unless I change them to a Duration)
    host_str: String, // Original user input, used for display
    pub host: SocketAddr,
    pings_sent: u32,
    latest_time: Option<u64>,
    sum_times: u64,
    sum_squared_times_ms: f64, // sum of the times squared, used for calculating jitter (std. dev of times)
    min_time: Option<u64>,
    max_time: Option<u64>,
    successful: u32,
    last_error: Option<ErrorKind>,
}

impl HostInfo {
    /// Creates a new HostInfo struct for the specified host. Host can be an IP address or domain name
    pub fn new(host: &str) -> Result<HostInfo, Error> {
        let possible_hosts = (host, 0).to_socket_addrs()?;
        let mut chosen_host: Option<SocketAddr> = None;
        
        for h in possible_hosts {
            // IPv6 isn't supported yet...
            if let V6(_) = h {
                continue;
            }
            // I guess we found one
            chosen_host = Some(h);
        }
        if chosen_host.is_none() {
            return Err(Error::from(ErrorKind::NotFound));
        }
        
        Ok(HostInfo {
            host_str: host.to_string(),
            host: chosen_host.unwrap(),
            pings_sent: 0,
            latest_time: None,
            sum_times: 0,
            sum_squared_times_ms: 0.0,
            min_time: None,
            max_time: None,
            successful: 0,
            last_error: None,
        })
    }
    
    pub fn average(&self) -> f32 {
        self.sum_times as f32 / (self.successful as f32 * 1000f32)
    }

    // Jitter is the standard deviation of latency
    pub fn jitter(&self) -> f32 {
        f32::sqrt((self.sum_squared_times_ms as f32 / (self.successful as f32)) - f32::powi(self.average(), 2))
    }
}

// Update for the messages passed from the worker threads
#[derive(Debug)]
pub enum StatusUpdate {
    Sent(usize),
    Received(usize, u64),
    Error(usize, ErrorKind),
}

pub fn update_host_info(update: &StatusUpdate, hinfos: &mut [HostInfo]) {
    match update {
        StatusUpdate::Sent(i) => {
            hinfos[*i].pings_sent += 1;
        },
        StatusUpdate::Received(i, latency) => {
            hinfos[*i].last_error = None;
            hinfos[*i].successful += 1;
            hinfos[*i].latest_time = Some(*latency);
            hinfos[*i].sum_times += *latency;
            let latency_ms: f64 = *latency as f64 / 1000f64; 
            hinfos[*i].sum_squared_times_ms += (latency_ms) * (latency_ms);
            if let Some(min) = hinfos[*i].min_time {
                if min > *latency {
                    hinfos[*i].min_time = Some(*latency);
                }
            } else {
                hinfos[*i].min_time = Some(*latency);
            }
            
            if let Some(max) = hinfos[*i].max_time {
                if max < *latency {
                    hinfos[*i].max_time = Some(*latency);
                }
            } else {
                hinfos[*i].max_time = Some(*latency);
            }
        },
        StatusUpdate::Error(i, errno) => {
            hinfos[*i].last_error = Some(*errno);
        }
    }
}

pub fn send_ping(host_info: &HostInfo, socket: &Socket) -> Result<(), Error> {
    // TODO: IPv6
    
    // Fill the buffer with the system time, then the numbers 0x10 to 0x37
    // (this is to mimic the packets of the ping(8) command)
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let secs = time.as_secs();
    let micros = time.subsec_nanos() as u64 / 1000;
    let mut buf: Vec<u8> = construct_echo_request(0xbeef, 1, &secs.to_be_bytes());
    buf.append(&mut micros.to_be_bytes().to_vec());
    buf.append(&mut (0x10_u8..=0x37_u8).collect());
    socket.send_to(&buf, &host_info.host.into())?;
    Ok(())
}

pub fn receive_ping(mut socket: &Socket) -> Result<(SocketAddr, u64), Error> {
    let mut rec_buf: [u8; 100] = [0; 100];
    let addr = socket.peek_sender()?;
    
    // Try to parse the received bytes
    if let Some(addr4) = addr.as_socket_ipv4() {
        let used_bytes = socket.read(&mut rec_buf)?;
        let maybe_message: Result<ICMPv4Message, IntoICMPv4MessageError> = rec_buf[..used_bytes].try_into();
        if let Ok(message) = maybe_message {
            let ts_seconds = u64::from_be_bytes(message.icmpv4_data[0..8].try_into().unwrap());
            let ts_sub_micros = u64::from_be_bytes(message.icmpv4_data[8..16].try_into().unwrap());
            let ts_micros = (ts_seconds as u128 * 1000000) + ts_sub_micros as u128;
            
            let cur_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let cur_micros = cur_time.as_nanos() / 1000;
            
            let diff_micros = cur_micros - ts_micros;
            
            return Ok((SocketAddr::V4(addr4), diff_micros as u64));
            
        } else if let Err(e) = maybe_message {
            print!("Error parsing response: ");
            match e {
                IntoICMPv4MessageError::UnknownType => println!("unknown type"),
                IntoICMPv4MessageError::UnknownCode => println!("unknown code"),
                IntoICMPv4MessageError::NotLongEnough => println!("message not long enough"),
                IntoICMPv4MessageError::OtherError => println!("other error"),
            }
        }
    }
    
    Err(Error::from(ErrorKind::NotFound))
}

pub fn mksocket() -> Result<Socket, Error> {
    let wildcard: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let socket = Socket::new(Domain::for_address(wildcard), Type::DGRAM, Some(Protocol::ICMPV4))?;
    Ok(socket)
}

const SEPARATOR: &str = " | ";

pub fn format_header(host_spaces: usize, stat_spaces: usize) -> String {
    let mut s = String::new();
    
    s.push_str(format!("{:<host_spaces$}", "Host").as_str());
    s.push_str(SEPARATOR);
    for heading in ["Time", "Minimum", "Average", "Maximum", "Jitter", "Loss"] {
        s.push_str(format!("{:<stat_spaces$}", heading).as_str());
        s.push_str(SEPARATOR);
    }
    
    s
}

pub fn format_host_info(host: &HostInfo, colour: bool, host_spaces: usize, stat_spaces: usize) -> String {
    let mut s = String::new();
    eprintln!("{:?}", host);
    
    s.push_str(format!("{:<host_spaces$}", host.host_str).as_str());
    s.push_str(SEPARATOR);
    
    if let Some(error) = host.last_error {
        for _x in 0..=stat_spaces - 6 {
            s.push(' ');
        }
        s.push_str(colour_error("Error", colour).as_str());
        s.push_str(": ");
        s.push_str(error.to_string().as_str());
        return s;
    }
    
    for stat in [to_sec(host.latest_time), to_sec(host.min_time), not_nan(host.average()), to_sec(host.max_time), not_nan(host.jitter())] {
        s.push_str(format_time_cell(colour, stat_spaces, stat).as_str());
        s.push_str(SEPARATOR);
    }
    s.push_str(format_colour_percent(colour, stat_spaces, host.successful, host.pings_sent).as_str());
    s.push_str(SEPARATOR);
    
    s
}

fn to_sec(microseconds: Option<u64>) -> Option<u64> {
    Some(microseconds? / 1000)
}

fn not_nan(num: f32) -> Option<u64> {
    if num.is_nan() {
        None
    } else {
        Some(num as u64)
    }
}

fn colour_error(msg: &str, colour: bool) -> String {
    if colour {
        style(msg).red().to_string()
    } else {
        msg.to_string()
    }
}

fn colour_ok(msg: &str, colour: bool) -> String {
    if colour {
        style(msg).green().to_string()
    } else {
        msg.to_string()
    }
}

fn colour_amber(msg: &str, colour: bool) -> String {
    if colour {
        style(msg).yellow().to_string()
    } else {
        msg.to_string()
    }
}

fn format_colour_percent(colour: bool, stat_spaces: usize, suc: u32, total: u32) -> String {
    let cell_string = format_percent_cell(stat_spaces, suc, total);
    if total == 0 || suc > total {
        return colour_error(&cell_string, colour);
    }
    
    let percent = (total - suc) * 100 / total;
    if !colour {
        return cell_string;
    }
    if percent < 10 {
        colour_ok(&cell_string, colour)
    } else if percent <= 50 {
        colour_amber(&cell_string, colour)
    } else { // greater than 50% loss (etc.)
        colour_error(&cell_string, colour)
    }
}

fn format_time_cell(colour: bool, stat_spaces: usize, stat: Option<u64>) -> String {
    let united_spaces = stat_spaces -  3;
    if let Some(s) = stat {
        let cell = format!("{:>united_spaces$} ms", s);
        if colour {
            return style(cell).blue().to_string();
        }
        cell
    } else {
        let cell = format!("{:>stat_spaces$}", "- ");
        if colour {
            return style(cell).red().to_string();
        }
        cell
    }
}

fn format_percent_cell(stat_spaces: usize, suc: u32, total: u32) -> String {
    let united_spaces = stat_spaces - 2;
    if total == 0 || suc > total {
        format!("{:>stat_spaces$}", "- ")
    } else {
        format!("{:>united_spaces$} %", ((total - suc) * 100) / total)
    }
}
