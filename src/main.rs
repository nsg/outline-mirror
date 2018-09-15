extern crate bufstream;

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread::spawn;
use bufstream::BufStream;
use std::env;
use std::process::{Command,Stdio};
use std::io::{BufReader,BufRead};

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
                     token: String,
                     whitelist: std::vec::Vec<String>,
                     insecure: String,
                     prefix: String) {

    let test_token = read(stream);

    if token.ne(&test_token) {
        println!("Incorrect TOKEN, I will close the connection.");
        writeln(stream, "Incorrent TOKEN");
        return
    }

    let _clone_repo = read(stream);
    let _repo_commit = read(stream);
    let command_no_prefix = String::from(read(stream).trim());
    let command = String::from(format!("{} {}", prefix, command_no_prefix));

    if !whitelist.contains(&command_no_prefix) && insecure == "0" {
        println!("Incorrect command \"{}\", I will close the connection.",
                 command_no_prefix);
        writeln(stream, format!("\"{}\" is not an valid command",
                command_no_prefix).as_str());
        return
    }

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

fn conf(var: &str, default: &str) -> String {
    env::var(var).unwrap_or(String::from(default))
}

fn parse_command(cmd: String) -> Vec<String> {
    let mut v = Vec::new();
    for c in cmd.split(",") {
        v.push(String::from(c));
    }
    v
}

fn main() {
    // Read configuration from environment
    let listen_addr = conf("LISTEN", "localhost");
    let listen_port = conf("PORT", "8080");
    let commands = conf("COMMANDS", "make,make check");
    let insecure = conf("INSECURE", "0");
    let prefix = conf("PREFIX", "");
    let token = conf("TOKEN", "insecure");

    let command_whitelist = parse_command(commands.clone());

    // Print startup messages
    println!("Welcome to Outline");
    println!("I will listen on {}, port {}", listen_addr, listen_port);
    println!("Allowed commands: {:?}", command_whitelist);
    if insecure.ne("0") {
        println!("WARNING! Insecure mode enabled, only use this for debug.");
    }
    if token.eq("insecure") {
        println!("WARNING! An insecure default token is set.");
    }
    if prefix.ne("") {
        println!("All commands will be prefixed with \"{}\"", prefix);
    }

    let listener = match TcpListener::bind(format!("{}:{}", listen_addr, listen_port)) {
        Ok(l) => l,
        Err(_) => panic!("TcpListener::bind failed!"),
    };

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let from = stream.peer_addr().unwrap().clone();
                let c = command_whitelist.clone();
                let i = insecure.clone();
                let p = prefix.clone();
                let t = token.clone();
                println!("Connection from {}", from);
                spawn(move|| {
                    let mut stream = BufStream::new(stream);
                    handle_connection(&mut stream, from, t, c, i, p);
                    println!("Close connection from {}", from);
                });
            },
            Err(_) => println!("Error in listener")
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn conf_test_default() {
        assert_eq!(super::conf("A", "B"), String::from("B"))
    }

    #[test]
    fn conf_test_found() {
        assert_eq!(super::conf("SHELL", "B"), String::from("/bin/bash"))
    }

    #[test]
    fn parse_command() {
        let cmd = String::from("a,b c,d");
        let mut v = Vec::new();
        v.push(String::from("a"));
        v.push(String::from("b c"));
        v.push(String::from("d"));
        assert_eq!(super::parse_command(cmd), v)
    }
}
