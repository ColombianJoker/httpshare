use clap::command;
use clap::Arg;
use clap::value_parser;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::BufReader;
use std::io::prelude::*;
use std::fs;
use std::path::Path;
use std::thread;
use std::thread::ThreadId;
use chrono::prelude::*;
use httpshare::ThreadPool;
use regex::Regex;

fn main() {
    let default_port = 7878_u16;
    let default_interface = String::from("localhost");
    let default_filename = String::from("index.html");
    let star = String::from("*");
    let all = String::from("0.0.0.0");
    let max_threads = 16;
    let max_reqs_worker = 1024;

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
    
    print!("[ Trying to bind to {bind_str}...");
    let listener = TcpListener::bind(bind_str).unwrap();
    println!(" done. ]");
    print!("[ Creating a thread pool to manage connection requests ...");
    let thread_pool = ThreadPool::new(max_threads);
    println!(" done. ]");
    
    // Process client connection requests
    for stream in listener.incoming().take(max_reqs_worker) {
      // take some requests and shutdown
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
  // let buffer = [0; 10240];
  let buf_reader = BufReader::new(&mut stream);
  let request_line = buf_reader.lines().next().unwrap().unwrap();
  let re = Regex::new(r"^(.*) (.*) (.*)$").unwrap(); // Capture three words or tokens
  let captures = re.captures(&request_line).unwrap();
  let request_pathname = captures.get(2).map_or("", |m| m.as_str());
    
  let full_filepath = format!("{}{}", filename, request_pathname);
  let now_str = Local::now().to_string();
  #[cfg(feature = "debug")]
  println!("{tid:?}< {:?} {request_pathname}={full_filepath}", now_str);
  #[cfg(not(feature = "debug"))]
  println!("{now_str} {full_filepath}");
  
  let mut http_code = "HTTP/1.1 404 NOT FOUND"; // by deafult gives 404
  let mut file_contents = String::new();
  
  if Path::new(&full_filepath).exists() {
    http_code = "HTTP/1.1 200 OK";              // if requested file exists then 200 OK
    file_contents = fs::read_to_string(&full_filepath).unwrap(); // load file
  }
  
  let content_length = file_contents.len();
  let http_response = format!("{http_code}\r\nContent-Length: {content_length}\r\n\r\n{file_contents}");
  
  stream.write_all(http_response.as_bytes()).unwrap();
}
