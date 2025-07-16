use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::io::Error;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use socket2::{Domain, Protocol, Socket, Type};

use crate::icmp::construct_echo_request;

mod icmp;

pub struct HostInfo {
    // Times in this struct are in milliseconds? (unless I change them to a Duration)
    host: SocketAddr,
    pings_sent: i32,
    latest_time: Option<i32>,
    sum_times: i64,
    min_time: Option<i32>,
    max_time: Option<i32>,
    successful: i32,
    last_error: Option<i32>, /* TODO: replace with errno or struct */
}

impl HostInfo {
    /// Creates a new HostInfo struct for the specified host. Host can be an IP address or domain name
    pub fn new(host: &str) -> Result<HostInfo, AddrParseError> {
        Ok(HostInfo {
            host: SocketAddr::new(host.parse()?, 0),
            pings_sent: 0,
            latest_time: None,
            sum_times: 0,
            min_time: None,
            max_time: None,
            successful: 0,
            last_error: None,
        })
    }
    
    pub fn average(&self) -> f32 {
        self.sum_times as f32 / self.pings_sent as f32
    }
    // TODO: jitter? (std. deviation of times)
}

pub fn ping_host(host_info: &mut HostInfo) -> Result<(), Error> {
    // TODO: persist sockets
    // TODO: IPv6
    let socket = Socket::new(Domain::for_address(host_info.host), Type::DGRAM, Some(Protocol::ICMPV4))?;
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let secs = time.as_secs();
    let nanos = time.subsec_nanos() as u64;
    let mut buf: Vec<u8> = construct_echo_request(0xbeef, 1, &secs.to_be_bytes());
    buf.append(&mut nanos.to_be_bytes().to_vec());
    buf.append(&mut ((0x10 as u8)..=(0x37 as u8)).collect());
    socket.send_to(&buf, &host_info.host.into())?;
    
    return Ok(())
}
