use std::fs;
use std::process::Command;

fn clone_repo_command(command: String, cwd: &str) -> bool {
    let mut args: Vec<&str> = command.split(" ").collect();

    let rcommand = Command::new(args.first().unwrap())
        .args(args.split_off(1))
        .env_clear()
        .current_dir(cwd)
        .output()
        .expect("Failed to execute process");

    println!("Execute: {}", command);
    if !(rcommand.status.success()) {
        println!("{}", String::from_utf8_lossy(&rcommand.stderr));
        return false
    }
    true
}

pub fn clone_repo(repo_path: String, repo_commit: String) {
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

    clone_repo_command(clone_command, "/tmp");
    clone_repo_command(checkout_command, clone_to_path);
}

#[cfg(test)]
mod tests {
    #[test]
    fn clone_repo_command() {
        let ok = super::clone_repo_command(String::from("ls /"), "/tmp");
        let notok = super::clone_repo_command(String::from("ls /dfgdfg"), "/tmp");
        assert_eq!(ok, true);
        assert_eq!(notok, false);
    }
}
