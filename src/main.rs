extern crate bufstream;
extern crate regex;

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread::spawn;
use bufstream::BufStream;
use std::{env, fs};
use std::process::{Command,Stdio};
use std::io::{BufReader,BufRead};
use regex::Regex;

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

fn clone_repo_command(command: String, cwd: &str) -> std::process::Output {
    let mut args: Vec<&str> = command.split(" ").collect();

    Command::new(args.first().unwrap())
        .args(args.split_off(1))
        .env_clear()
        .current_dir(cwd)
        .output()
        .expect("Failed to execute process")
}

fn clone_repo(repo_path: String, repo_commit: String) {
    let clone_to_path = "/tmp/outline_workdir";

    match fs::remove_dir_all(clone_to_path) {
        Ok(_) => println!("Remove {}", clone_to_path),
        Err(_) => println!("{} was not there", clone_to_path),
    }

    let clone_command = format!(
        "git clone --depth 128 --recurse-submodules \
        --quiet --shallow-submodules \
        {} {}", repo_path, clone_to_path);
    let checkout_command = format!("git checkout {}", repo_commit);

    {
        println!("Execute: {}", clone_command);
        let command = clone_repo_command(clone_command, "/tmp");
        if !command.status.success() {
            println!("{}", String::from_utf8_lossy(&command.stderr));
        }
    }

    {
        println!("Execute: {}", checkout_command);
        let command = clone_repo_command(checkout_command, clone_to_path);
        if !command.status.success() {
            println!("{}", String::from_utf8_lossy(&command.stderr));
        }
    }

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

    let repo_path = read(stream);
    let repo_commit = read(stream);
    let command_no_prefix = String::from(read(stream).trim());
    let command = String::from(format!("{} {}", prefix, command_no_prefix));

    if !whitelist.contains(&command_no_prefix) && insecure == "0" {
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

    clone_repo(repo_path, String::from(repo_commit));

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
