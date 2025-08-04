use std::mem::MaybeUninit;
use std::net::{AddrParseError, IpAddr, SocketAddr, SocketAddrV4};
use std::io::{self, Error, Read};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use socket2::{Domain, Protocol, Socket, Type};

use crate::icmp::{construct_echo_request, ICMPv4Message, IntoICMPv4MessageError};

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
    // (I think I'd need to store the sum of the times squared for the std. dev formula)
}

pub fn send_ping(host_info: &mut HostInfo, socket: &Socket) -> Result<(), Error> {
    // TODO: IPv6
    
    // Fill the buffer with the system time, then the numbers 0x10 to 0x37
    // (this is to mimic the packets of the ping(8) command)
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let secs = time.as_secs();
    let micros = (time.subsec_nanos() as u64 / 1000);
    let mut buf: Vec<u8> = construct_echo_request(0xbeef, 1, &secs.to_be_bytes());
    buf.append(&mut micros.to_be_bytes().to_vec());
    buf.append(&mut ((0x10 as u8)..=(0x37 as u8)).collect());
    socket.send_to(&buf, &host_info.host.into())?;
    return Ok(())
}

pub fn receive_ping(host_info: &HostInfo, mut socket: &Socket) -> Result<(), Error> {
    let mut rec_buf: [u8; 100] = [0; 100];
    let addr = socket.peek_sender()?;
    let used_bytes = socket.read(&mut rec_buf)?;
    
    // Try to parse the received bytes
    if let Some(addr4) = addr.as_socket_ipv4() {
        if SocketAddr::V4(addr4) == host_info.host {
            println!("Yay, got a reply from the right host");
            let maybe_message: Result<ICMPv4Message, IntoICMPv4MessageError> = rec_buf[..used_bytes].try_into();
            if let Ok(message) = maybe_message {
                println!("Message received: {:?}", message);
                // try to parse the timestamp in the received packet
                let ts_seconds = u64::from_be_bytes(message.icmpv4_data[0..8].try_into().unwrap());
                let ts_sub_micros = u64::from_be_bytes(message.icmpv4_data[8..16].try_into().unwrap());
                println!("ts_seconds: {}, ts_sub_micros: {}", ts_seconds, ts_sub_micros);
                let ts_micros = (ts_seconds as u128 * 1000000) + ts_sub_micros as u128;
                
                let cur_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
                let cur_micros = cur_time.as_nanos() / 1000;
                
                println!("ts_micros: {}, cur_micros: {}", ts_micros, cur_micros);
                let diff_micros = cur_micros - ts_micros;
                println!("Latency: {}s", diff_micros as f64 / 1000000.0);
                
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
    }
    
    return Ok(())
}

pub fn mksocket(host_info: &HostInfo) -> Result<Socket, Error> {
    let mut socket = Socket::new(Domain::for_address(host_info.host), Type::DGRAM, Some(Protocol::ICMPV4))?;
    socket.set_read_timeout(Some(Duration::from_secs(2)));
    Ok(socket)
}
