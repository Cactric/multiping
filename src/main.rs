use console::{Term, style};
use socket2::{Domain, Protocol, Socket, Type};
use std::{io::Error, net::SocketAddr, process::exit};
use clap::Parser;
use std::net::IpAddr;
use std::time::Duration;
use std::sync::mpsc;
use std::thread;
use rand::random;

use multiping::{mksocket, receive_ping, send_ping, update_host_info, HostInfo, StatusUpdate};

mod icmp;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Which hosts (IP addresses or domain names) to ping
    hosts: Vec<String>,
    
    /// How often the hosts should be pinged
    #[arg(short = 'i', long, default_value_t = 1.0)]
    interval: f32,
    
    /// If specified, min, max and average latency stats are shown
    #[arg(short, long)]
    statistics: bool,
}

fn say_hello() -> Result<(),Error> {
    let term = Term::stdout();
    term.clear_screen()?;
    term.write_line(&style("hello!").cyan().to_string())?;
    Ok(())
}

fn main() {
    //let _ = say_hello();
    
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
    for h in args.hosts {
        let mut maybe_hinfo = HostInfo::new(&h);
        if let Ok(mut hinfo) = maybe_hinfo {
            hinfos.push(hinfo);
            /*let mut socket = mksocket(&hinfo).unwrap();
            match send_ping(&hinfo, &tx, &socket) {
                Ok(()) => {
                    println!("Pinging {} succeeded.", h);
                    match receive_ping(&hinfo, &tx, &socket) {
                        Ok(()) => println!("And getting a response succeeded!"),
                        Err(e) => println!("Getting a response from {} failed: {}", h, e),
                    }
                },
                Err(e) => println!("Pinging {} failed: {}.", h, e),
            }*/
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
                    send_tx.send(StatusUpdate::Error(i, 0)).unwrap();
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
                    for (i, h) in recv_enum_host_infos.clone() {
                        if (h.host == addr) {
                            recv_tx.send(StatusUpdate::Received(i, latency)).unwrap();
                            break;
                        }
                    }
                },
                Err(e) => {
                    //recv_tx.send(StatusUpdate::Error(i, 0)).unwrap();
                }
            }
            thread::sleep(Duration::from_secs_f32(args.interval));
        }
    });
    
    // Listen for updates
    for update in rx {
        println!("Update: {:?}", update);
        update_host_info(&update, &mut hinfos);
    }
}
