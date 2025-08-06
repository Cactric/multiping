use std::mem::MaybeUninit;
use std::net::{AddrParseError, IpAddr, SocketAddr, SocketAddrV4};
use std::io::{self, Error, Read, ErrorKind};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::sync::mpsc;
use socket2::{Domain, Protocol, Socket, Type};

use crate::icmp::{construct_echo_request, ICMPv4Message, IntoICMPv4MessageError};

mod icmp;

#[derive(Clone, Debug)]
pub struct HostInfo {
    // Times in this struct are in milliseconds? (unless I change them to a Duration)
    pub host: SocketAddr,
    pings_sent: u32,
    latest_time: Option<u64>,
    sum_times: u64,
    min_time: Option<u64>,
    max_time: Option<u64>,
    successful: u32,
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

// Update for the messages passed from the worker threads
#[derive(Debug)]
pub enum StatusUpdate {
    Sent(usize),
    Received(usize, u64),
    Error(usize, i32), // TODO: replace u32 with an error type
}

pub fn update_host_info(update: &StatusUpdate, hinfos: &mut Vec<HostInfo>) {
    match update {
        StatusUpdate::Sent(i) => {
            hinfos[*i].pings_sent += 1;
        },
        StatusUpdate::Received(i, latency) => {
            hinfos[*i].last_error = None;
            hinfos[*i].successful += 1;
            hinfos[*i].latest_time = Some(*latency);
            hinfos[*i].sum_times += *latency;
            if let Some(min) = hinfos[*i].min_time {
                if min > *latency {
                    hinfos[*i].min_time = Some(*latency);
                }
            } else {
                hinfos[*i].min_time = Some(*latency);
            }
            
            if let Some(max) = hinfos[*i].max_time {
                if max > *latency {
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
    let micros = (time.subsec_nanos() as u64 / 1000);
    let mut buf: Vec<u8> = construct_echo_request(0xbeef, 1, &secs.to_be_bytes());
    buf.append(&mut micros.to_be_bytes().to_vec());
    buf.append(&mut ((0x10 as u8)..=(0x37 as u8)).collect());
    socket.send_to(&buf, &host_info.host.into())?;
    return Ok(())
}

pub fn receive_ping(mut socket: &Socket) -> Result<(SocketAddr, u64), Error> {
    let mut rec_buf: [u8; 100] = [0; 100];
    let addr = socket.peek_sender()?;
    
    // Try to parse the received bytes
    if let Some(addr4) = addr.as_socket_ipv4() {
        let used_bytes = socket.read(&mut rec_buf)?;
        // println!("Yay, got a reply from the right host");
        let maybe_message: Result<ICMPv4Message, IntoICMPv4MessageError> = rec_buf[..used_bytes].try_into();
        if let Ok(message) = maybe_message {
            // println!("Message received: {:?}", message);
            // try to parse the timestamp in the received packet
            let ts_seconds = u64::from_be_bytes(message.icmpv4_data[0..8].try_into().unwrap());
            let ts_sub_micros = u64::from_be_bytes(message.icmpv4_data[8..16].try_into().unwrap());
            // println!("ts_seconds: {}, ts_sub_micros: {}", ts_seconds, ts_sub_micros);
            let ts_micros = (ts_seconds as u128 * 1000000) + ts_sub_micros as u128;
            
            let cur_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let cur_micros = cur_time.as_nanos() / 1000;
            
            // println!("ts_micros: {}, cur_micros: {}", ts_micros, cur_micros);
            let diff_micros = cur_micros - ts_micros;
            // println!("Latency: {}s", diff_micros as f64 / 1000000.0);
            
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
    let mut socket = Socket::new(Domain::for_address(wildcard), Type::DGRAM, Some(Protocol::ICMPV4))?;
    socket.set_read_timeout(Some(Duration::from_secs(2)));
    Ok(socket)
}
