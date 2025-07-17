use std::mem::MaybeUninit;
use std::net::{AddrParseError, IpAddr, SocketAddr, SocketAddrV4};
use std::io::{self, Error, Read};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use socket2::{Domain, Protocol, Socket, Type};

use crate::icmp::{construct_echo_request, ICMPv4Message};

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

pub fn ping_host(host_info: &mut HostInfo, mut socket: &Socket) -> Result<(), Error> {
    // TODO: IPv6
    
    // Fill the buffer with the system time, then the numbers 0x10 to 0x37
    // (this is to mimic the packets of the ping(8) command)
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let secs = time.as_secs();
    let nanos = time.subsec_nanos() as u64;
    let mut buf: Vec<u8> = construct_echo_request(0xbeef, 1, &secs.to_be_bytes());
    buf.append(&mut nanos.to_be_bytes().to_vec());
    buf.append(&mut ((0x10 as u8)..=(0x37 as u8)).collect());
    socket.send_to(&buf, &host_info.host.into())?;
    
    // Echo request sent, time to wait for a reply...
    let mut rec_buf: [u8; 100] = [0; 100];
    let addr = socket.peek_sender()?;
    let used_bytes = socket.read(&mut rec_buf)?;
    
    // Try to parse the received bytes
    if let Some(addr4) = addr.as_socket_ipv4() {
        if SocketAddr::V4(addr4) == host_info.host {
            println!("Yay, got a reply from the right host");
        }
    }
    
    return Ok(())
}

pub fn mksocket(host_info: &HostInfo) -> Result<Socket, Error> {
    let mut socket = Socket::new(Domain::for_address(host_info.host), Type::DGRAM, Some(Protocol::ICMPV4))?;
    socket.set_read_timeout(Some(Duration::from_secs(2)));
    Ok(socket)
}
