use std::net::SocketAddr;
use std::io::{Error, Read, ErrorKind};
use std::time::SystemTime;
use std::net::ToSocketAddrs;
use socket2::{Domain, Protocol, Socket, Type};

use crate::icmp::*;

pub mod icmp;

#[derive(Clone, Debug)]
pub struct HostInfo {
    // Times in this struct are in microseconds? (unless I change them to a Duration)
    pub host_str: String, // Original user input, used for display
    pub host: SocketAddr,
    pub pings_sent: u32,
    pub latest_time: Option<u64>,
    pub sum_times: u64,
    pub sum_squared_times_ms: f64, // sum of the times squared, used for calculating jitter (std. dev of times)
    pub min_time: Option<u64>,
    pub max_time: Option<u64>,
    pub successful: u32,
    pub last_error: Option<ErrorKind>,
}

impl HostInfo {
    /// Creates a new HostInfo struct for the specified host. Host can be an IP address or domain name
    pub fn new(host: &str) -> Result<HostInfo, Error> {
        let possible_hosts = (host, 0).to_socket_addrs()?;
        let mut chosen_host: Option<SocketAddr> = None;
        
        for h in possible_hosts {
            // I guess we found one
            // TODO: choosing logic? Or at least have options to restrict to v4/v6
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
    // Fill the buffer with the system time, then the numbers 0x10 to 0x37
    // (this is to mimic the packets of the ping(8) command)
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let secs = time.as_secs();
    let micros = time.subsec_nanos() as u64 / 1000;
    let mut buf: Vec<u8>;
    if host_info.host.is_ipv4() {
        buf = construct_echo_request_v4(0xbeef, 1, &secs.to_be_bytes());
    } else if host_info.host.is_ipv6() {
        buf = construct_echo_request_v6(0xcafe, 1, &secs.to_be_bytes());
    } else {
        return Err(ErrorKind::AddrNotAvailable.into());
    }
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
        let maybe_message: Result<ICMPv4Message, IntoICMPError> = rec_buf[..used_bytes].try_into();
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
                IntoICMPError::UnknownType => println!("unknown type"),
                IntoICMPError::UnknownCode => println!("unknown code"),
                IntoICMPError::NotLongEnough => println!("message not long enough"),
                IntoICMPError::OtherError => println!("other error"),
            }
        }
    } else if let Some(addr6) = addr.as_socket_ipv6() {
        let used_bytes = socket.read(&mut rec_buf)?;
        let maybe_message: Result<ICMPv6Message, IntoICMPError> = rec_buf[..used_bytes].try_into();
        if let Ok(message) = maybe_message {
            let ts_seconds = u64::from_be_bytes(message.body[0..8].try_into().unwrap());
            let ts_sub_micros = u64::from_be_bytes(message.body[8..16].try_into().unwrap());
            let ts_micros = (ts_seconds as u128 * 1000000) + ts_sub_micros as u128;
            
            let cur_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let cur_micros = cur_time.as_nanos() / 1000;
            
            let diff_micros = cur_micros - ts_micros;
            
            return Ok((SocketAddr::V6(addr6), diff_micros as u64));
            
        } else if let Err(e) = maybe_message {
            print!("Error parsing response: ");
            match e {
                IntoICMPError::UnknownType => println!("unknown type"),
                IntoICMPError::UnknownCode => println!("unknown code"),
                IntoICMPError::NotLongEnough => println!("message not long enough"),
                IntoICMPError::OtherError => println!("other error"),
            }
        }
    }
    
    Err(Error::from(ErrorKind::NotFound))
}

pub fn mkv4socket() -> Result<Socket, Error> {
    let wildcard: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let socket = Socket::new(Domain::for_address(wildcard), Type::DGRAM, Some(Protocol::ICMPV4))?;
    Ok(socket)
}

pub fn mkv6socket() -> Result<Socket, Error> {
    let wildcard: SocketAddr = "[::]:0".parse().unwrap();
    let socket = Socket::new(Domain::for_address(wildcard), Type::DGRAM, Some(Protocol::ICMPV6))?;
    Ok(socket)
}
