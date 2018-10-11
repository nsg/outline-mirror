extern crate bufstream;
extern crate regex;

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread::spawn;
use bufstream::BufStream;
use std::process::{Command,Stdio};
use std::io::{BufReader,BufRead};
use regex::Regex;

mod git;
mod config;

fn read(stream: &mut BufStream<TcpStream>) -> String {
    let mut reads = String::new();
    stream.read_line(&mut reads).unwrap();
    String::from(reads.trim())
}

fn write(stream: &mut BufStream<TcpStream>, msg: &str) {
    stream.write(msg.as_bytes()).expect("Unable to write to socker!");
    stream.flush().expect("Unable to flush the socket!")
}

fn writeln(stream: &mut BufStream<TcpStream>, msg: &str) {
    write(stream, format!("{}\n", msg).as_str());
}

fn handle_connection(stream: &mut BufStream<TcpStream>,
                     from: std::net::SocketAddr,
                     config: config::Config) {

    let test_token = read(stream);

    if config.token.ne(&test_token) {
        println!("Incorrect TOKEN, I will close the connection.");
        writeln(stream, "Incorrent TOKEN");
        return
    }

    let repo_path = read(stream);
    let repo_commit = read(stream);
    let command_no_prefix = String::from(read(stream).trim());
    let command = String::from(format!("{} {}", config.prefix, command_no_prefix));

    if !config.commands.contains(&command_no_prefix) && config.insecure == "0" {
        println!("Incorrect command \"{}\", I will close the connection.",
                 command_no_prefix);
        writeln(stream, format!("\"{}\" is not an valid command",
                command_no_prefix).as_str());
        return
    }

    let sha1hash = Regex::new(r"^[a-f0-9]{40}$").unwrap();
    if !sha1hash.is_match(repo_commit.trim()) {
        println!("Invalid hash specified, close the connection.");
        println!("{}", repo_commit.trim());
        writeln(stream, "You need to specify a full sha1 commit hash matching a-z0-9");
        return
    }

    let cloneurl_regexp =
        r"^((git|ssh|http(s)?)|(git@[\w\.]+))(:(//)?)([\w\.@:/\-~]+)(\.git)(/)?$";
    let cloneurl = Regex::new(cloneurl_regexp).unwrap();
    if !cloneurl.is_match(repo_path.trim()) {
        println!("Invalid repo url, close the connection.");
        println!("{}", repo_path.trim());
        writeln(stream, "Unsupported clone url format.");
        return
    }

    git::clone_repo(repo_path, String::from(repo_commit));

    let mut args: Vec<&str> = command.split(" ").collect();

    let mut child = Command::new(args.first().unwrap())
        .args(args.split_off(1))
        .env_clear()
        .current_dir("/tmp")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()
        .expect("Failed to execute process");

    let pid = child.id();
    println!("[{}] Spawned child process {}", from, pid);

    let mut buf_stdout = BufReader::new(child.stdout.take().unwrap());
    let mut buf_stderr = BufReader::new(child.stderr.take().unwrap());
    let mut buffer = String::new();

    let mut x = 1;
    while x > 0 {
        let a = buf_stdout.read_line(&mut buffer).unwrap();
        let b = buf_stderr.read_line(&mut buffer).unwrap();
        x = a + b;

        match stream.write(buffer.as_bytes()) {
            Ok(_) => (),
            Err(_) => {
                println!("[{}] Broken pipe, unable to write to socket, kill pid {}",
                         from, pid);
                child.kill().expect("The command wasn't running");
                return
            }
        }

        match stream.flush() {
            Ok(_) => (),
            Err(_) => {
                println!("[{}] Broken pipe, unable to write to socket, kill pid {}",
                         from, pid);
                child.kill().expect("The command wasn't running");
                return
            }
        }

        buffer.clear();
    }
}

fn main() {
    // Read configuration from environment
    let config = config::read_conf();

    // Print startup messages
    println!("Welcome to Outline");
    println!("I will listen on {}, port {}", config.listen_addr, config.listen_port);
    println!("Allowed commands: {:?}", config.commands);

    if config.insecure.ne("0") {
        println!("WARNING! Insecure mode enabled, only use this for debug.");
    }
    if config.token.eq("insecure") {
        println!("WARNING! An insecure default token is set.");
    }
    if config.prefix.ne("") {
        println!("All commands will be prefixed with \"{}\"", config.prefix);
    }

    let listener = match TcpListener::bind(format!("{}:{}",
                                                   config.listen_addr,
                                                   config.listen_port)) {
        Ok(l) => l,
        Err(_) => panic!("TcpListener::bind failed!"),
    };

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let from = stream.peer_addr().unwrap().clone();
                println!("Connection from {}", from);
                let c = config.clone();
                spawn(move|| {
                    let mut stream = BufStream::new(stream);
                    handle_connection(&mut stream, from, c);
                    println!("Close connection from {}", from);
                });
            },
            Err(_) => println!("Error in listener")
        }
    }
}
