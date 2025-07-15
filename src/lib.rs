use std::net::SocketAddr;
use std::io::Error;
use socket2::{Domain, Protocol, Socket, Type};

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
    pub fn average(&self) -> f32 {
        self.sum_times as f32 / self.pings_sent as f32
    }
    // TODO: jitter? (std. deviation of times)
}

pub fn ping_host(host_info: &mut HostInfo) -> Result<(), Error> {
    // TODO: persist sockets
    // TODO: IPv6
    let socket = Socket::new(Domain::for_address(host_info.host), Type::DGRAM, Some(Protocol::ICMPV4))?;
    socket.bind(&host_info.host.into())?;
    //socket.listen(128)?;
    
    return Ok(())
}
