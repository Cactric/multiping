use console::{Term, style};
use socket2::{Domain, Protocol, Socket, Type};
use std::{io::{Error, Write}, net::SocketAddr, process::exit};
use clap::Parser;
use std::net::IpAddr;
use std::time::Duration;
use std::sync::mpsc;
use std::thread;
use rand::random;

use multiping::{mksocket, receive_ping, send_ping, update_host_info, format_host_info, format_header, HostInfo, StatusUpdate};

mod icmp;

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

    if args.hosts.len() < 1 {
        eprintln!("You need to specify hosts on the command line.\nExample: multiping 127.0.0.1");
        exit(1);
    }
    
    let (send_tx, rx) = mpsc::channel::<StatusUpdate>();
    let recv_tx = send_tx.clone();
    let mut hinfos: Vec<HostInfo> = Vec::new();
    
    // Parse the provided hosts into a vector of HostInfos
    for h in &args.hosts {
        let mut maybe_hinfo = HostInfo::new(&h);
        if let Ok(mut hinfo) = maybe_hinfo {
            hinfos.push(hinfo);
        } else {
            eprintln!("Failed to parse {}", h);
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
                        if (h.host == addr) {
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
    
    let mut term = Term::buffered_stdout();
    term.hide_cursor();
    
    // Listen for updates
    for update in rx {
        update_host_info(&update, &mut hinfos);
        update_display(&term, &hinfos, args.colour.unwrap_or(true));
    }
    
    term.show_cursor();
    term.flush();
}

fn update_display(term: &Term, hinfos: &Vec<HostInfo>, colour: bool) -> Result<(), Error> {
    term.clear_screen()?;
    
    let host_spaces = 19;
    let stat_spaces = 8;
    
    let header_line = format_header(host_spaces, stat_spaces);
    term.write_line(header_line.as_str())?;
    
    for host in hinfos {
        let line = format_host_info(host, colour, host_spaces, stat_spaces);
        term.write_line(line.as_str())?;
    }
    
    term.show_cursor()?;
    term.flush();
    
    Ok(())
}
