use clap::command;
use clap::Arg;
use clap::value_parser;
use std::net::TcpListener;
// use std::net::ToSocketAddrs;
use std::net::TcpStream;
use std::io;
use std::io::prelude::*;
use std::fs;
use std::thread;
use std::thread::ThreadId;
use std::time::Duration;
use httpshare::ThreadPool;

fn main() {
    let default_port = 7878_u16;
    let default_interface = String::from("localhost");
    let default_filename = String::from("index.html");
    let star = String::from("*");
    let all = String::from("0.0.0.0");
    let max_threads = 16;

    // Process command line options and arguments
    let match_result = command!()
    .about("Starts a simple HTTP server in a directory to share it"
    ).arg(
      Arg::new("interface")
        .short('i')
        .long("interface")
        .aliases(["if"])
        .required(false)
        .help("Interface to bind to (defaults to '{default_interface}')")
    ).arg(
      Arg::new("port")
        .short('p')
        .long("port")
        .aliases(["tcpport","tcp-port"])
        .required(false)
        .value_parser(value_parser!(u16))
        .help("TCP port to listen to (defaults to {default_port})")
    ).arg(
      Arg::new("quiet")
        .short('q')
        .long("quiet")
        .required(false)
        .num_args(0)
        .help("If to show basic messages")
    ).arg(
      Arg::new("filename")
        .required(true)
        .help("File to share")
    ).get_matches();
    // Process command line options and arguments
    
    let quiet = match_result.get_one::<bool>("quiet").unwrap();
    let mut interface = match_result.get_one::<String>("interface").unwrap_or(&default_interface);
    if *interface == star {
      interface = &all;
      if !quiet {
        println!("[* Changed interface to {interface} *]");
      }
    }
    let port = match_result.get_one::<u16>("port").unwrap_or(&default_port);
    let filename = match_result.get_one::<String>("filename").unwrap_or(&default_filename).clone();
    if !quiet {
      println!("| Verbose: {}", !quiet);
      println!("| Interface: {}", interface);
      println!("| Port: {}", port);
      println!("| Filename: {}", filename);
    }
    let bind_str = format!("{interface}:{port}");
    // let sock_addr = bind_str.to_socket_addrs().unwrap();
    // let bind_String = String::from(bind_str);
    
    print!("[ Trying to bind to {bind_str}...");
    let listener = TcpListener::bind(bind_str).unwrap();
    println!(" done. ]\n");
    print!("[ Creating a thread pool to manage connection requests ...");
    let thread_pool = ThreadPool::new(max_threads);
    println!(" done. ]\n");
    
    // Process client connection requests
    for stream in listener.incoming() {
      let stream = stream.unwrap();
      let fname = filename.clone();
      thread_pool.execute(move || { 
        let tid=thread::current().id();
        handle_connection(stream, tid, fname);
      });
    }
    // Process client connection requests
}

fn handle_connection(mut stream: TcpStream, tid: ThreadId, filename: String) {
  let mut buffer = [0; 10240];
  let get = b"GET / HTTP/1.1\r\n";
  let sleep = b"GET /sleep HTTP/1.1\r\n";
  let notfound_filename = String::from("404.html");
  let mut sent_filename = String::new();
  let mut contents = String::new();
  let mut response = String::new();

  stream.read(&mut buffer).unwrap();

  if buffer.starts_with(get) {
    contents = fs::read_to_string(filename.clone()).unwrap();
    response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", contents.len(), contents);
    sent_filename = filename.clone();
  } else if buffer.starts_with(sleep) {
    thread::sleep(Duration::from_secs(5)); // wait 5 secs
    contents = fs::read_to_string(filename.clone()).unwrap();
    response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", contents.len(), contents);
    sent_filename = filename.clone();
  } else {
    contents = fs::read_to_string(notfound_filename.clone()).unwrap();
    response = format!("HTTP/1.1 404 NOT FOUND\r\nContent-Length: {}\r\n\r\n{}", contents.len(), contents);
    sent_filename = notfound_filename.clone();
  }
  // if !quiet {
  //   print!("{tid:?}< Request: {}", String::from_utf8_lossy(&buffer[..])); // report request
  // }
  stream.write(response.as_bytes()).unwrap();
  stream.flush().unwrap();
  // if !quiet {
  //   println!("{tid:?}> {} contents ({} bytes) sent.", sent_filename.clone(), contents.len());  // report response
  // }
}
