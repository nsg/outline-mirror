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

fn handle_connection(stream: &mut BufStream<TcpStream>,
                     from: std::net::SocketAddr,
                     _whitelist: std::vec::Vec<String>,
                     _insecure: String,
                     _prefix: String) {

    stream.write(b"You are connected to Outline, you should know what to do\n").unwrap();
    stream.flush().unwrap();

    let clone_repo = read(stream);
    let repo_commit = read(stream);
    let run_command = read(stream);

    println!("[{}] REPO: {}", from, clone_repo);
    println!("[{}] COMMIT: {}", from, repo_commit);
    println!("[{}] COMMAND: {}", from, run_command);

    let mut child = Command::new("/tmp/bar")
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

    let command_whitelist = parse_command(commands.clone());
    println!("COMMAND WHITELIST: {:?}", command_whitelist);

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
                println!("Connection from {}", from);
                spawn(move|| {
                    let mut stream = BufStream::new(stream);
                    handle_connection(&mut stream, from, c, i, p);
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
