pub struct HostInfo {
    // Times in this struct are in milliseconds? (unless I change them to a Duration)
    host: String,
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

pub fn ping_host(host_info: &mut HostInfo) {
    // Pretend the ping works first and just modify the struct
    host_info.pings_sent += 1;
    host_info.sum_times += 200;
    host_info.successful += 1;
}
