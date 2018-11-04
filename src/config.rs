use std::env;

#[derive(Clone)]
pub struct Config {
    pub listen_addr: String,
    pub listen_port: String,
    pub commands: Vec<String>,
    pub insecure: String,
    pub prefix: String,
    pub token: String
}

fn parse_command(cmd: String) -> Vec<String> {
    let mut v = Vec::new();
    for c in cmd.split(",") {
        v.push(String::from(c));
    }
    v
}

fn conf(var: &str, default: &str) -> String {
    env::var(var).unwrap_or(String::from(default))
}

pub fn read_conf() -> Config {
    Config {
        listen_addr: conf("LISTEN", "localhost"),
        listen_port: conf("PORT", "8080"),
        commands: parse_command(conf("COMMANDS", "make,make check")),
        insecure: conf("INSECURE", "0"),
        prefix: conf("PREFIX", ""),
        token: conf("TOKEN", "insecure")
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    #[test]
    fn conf_test_default() {
        assert_eq!(super::conf("A", "B"), String::from("B"))
    }

    #[test]
    fn conf_test_found() {
        env::set_var("SHELL", "/bin/bash");
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
