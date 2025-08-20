use console::{Term, style};
use std::{cmp::max, io::Error, process::exit};
use clap::Parser;
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use multiping::*;

pub mod icmp;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Which hosts (IP addresses or domain names) to ping
    hosts: Vec<String>,
    
    /// How often the hosts should be pinged (in seconds)
    #[arg(short = 'i', long, default_value_t = 1.0)]
    interval: f32,
    
    /// Whether colours are used in the output
    #[arg(short = 'c', long)]
    colour: Option<bool>,
}

fn main() {
    // Parse arguments
    let args = Arguments::parse();

    if args.hosts.is_empty() {
        eprintln!("You need to specify hosts on the command line.\nExample: multiping 127.0.0.1");
        exit(1);
    }
    
    let (send_tx, rx) = mpsc::channel::<StatusUpdate>();
    let recv_tx = send_tx.clone();
    let mut hinfos: Vec<HostInfo> = Vec::new();
    let mut max_host_width = 0;
    
    // Parse the provided hosts into a vector of HostInfos
    for h in &args.hosts {
        let maybe_hinfo = HostInfo::new(h);
        if let Ok(hinfo) = maybe_hinfo {
            hinfos.push(hinfo);
            max_host_width = max(max_host_width, console::measure_text_width(h));
        } else {
            eprintln!("Failed to parse {}", h);
            exit(1);
        }
    }
    
    let recv_enum_host_infos = hinfos.clone().into_iter().enumerate();
    let send_enum_host_infos = hinfos.clone().into_iter().enumerate();
    let socket = mksocket().unwrap();
    let socket2 = socket.try_clone().unwrap();
    
    // Spawn threads
    thread::spawn(move || {
        loop {
            for (i, h) in send_enum_host_infos.clone() {
                //println!("Host: {:?}", h.1.host);
                if let Err(e) = send_ping(&h, &socket) {
                    // Error
                    send_tx.send(StatusUpdate::Error(i, e.kind())).unwrap();
                } else {
                    send_tx.send(StatusUpdate::Sent(i)).unwrap();
                }
            }
            thread::sleep(Duration::from_secs_f32(args.interval));
        }
    });
    thread::spawn(move || {
        loop {
            match receive_ping(&socket2) {
                Ok((addr, latency)) => {
                    println!("Latency from {:?}: {}", &addr, &latency);
                    // Figure out which host the address was from
                    let mut found = false;
                    for (i, h) in recv_enum_host_infos.clone() {
                        if h.host == addr {
                            recv_tx.send(StatusUpdate::Received(i, latency)).unwrap();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        eprintln!("Host not found: addr = {}", addr)
                    }
                },
                Err(e) => {
                    //recv_tx.send(StatusUpdate::Error(i, e.kind())).unwrap();
                    eprintln!("Error listening to socket: {}", e);
                }
            }
        }
    });
    
    if let Err(e) = display_loop(rx, hinfos, max_host_width, args) {
        eprintln!("Error in display loop {}", e);
    }
}

fn display_loop(rx: Receiver<StatusUpdate>, mut hinfos: Vec<HostInfo>, max_host_width: usize, args: Arguments) -> Result<(), Error> {
    let term = Term::buffered_stdout();
    let colour = console::colors_enabled() || args.colour.unwrap_or(false);
    term.hide_cursor()?;
    
    // Listen for updates
    for update in rx {
        update_host_info(&update, &mut hinfos);
        update_display(&term, &hinfos, max_host_width, colour)?;
    }
    
    term.show_cursor()?;
    Ok(())
}

fn update_display(term: &Term, hinfos: &Vec<HostInfo>, max_host_width: usize, colour: bool) -> Result<(), Error> {
    term.clear_screen()?;
    
    let host_spaces = max(12, max_host_width);
    let stat_spaces = 7;
    
    let header_line = format_header(host_spaces, stat_spaces);
    term.write_line(header_line.as_str())?;
    
    for host in hinfos {
        let line = format_host_info(host, colour, host_spaces, stat_spaces);
        term.write_line(line.as_str())?;
    }
    
    term.show_cursor()?;
    term.flush()?;
    
    Ok(())
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
