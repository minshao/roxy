use super::run_command;
use anyhow::Result;
use std::{
    fmt::Write as FmtWrite,
    fs::{self, OpenOptions},
    io::Write as IoWrite,
};

const SSHD_CONFIG: &str = "/etc/ssh/sshd_config";
const SSHD_DEFAULT_PORT: u16 = 22;

// Sets sshd port.
//
// # Example
//
// let ret = sshd::set("10022")?;
//
// # Errors
//
// * invalid port
// * fail to open ``/etc/ssh/sshd_config``
// * fail to write modified contents to ``/etc/ssh/sshd_config``
// * fail to restart sshd service
pub(crate) fn set(port: &str) -> Result<bool> {
    let port = port.parse::<u16>()?;

    let contents = fs::read_to_string(SSHD_CONFIG)?;
    let lines = contents.lines();
    let mut new_contents = String::new();
    for line in lines {
        if !line.starts_with("Port ") {
            new_contents.push_str(line);
            new_contents.push('\n');
        }
    }

    writeln!(new_contents, "Port {}", port).expect("writing to string should not fail");

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(SSHD_CONFIG)?;

    file.write_all(new_contents.as_bytes())?;

    run_command("systemctl", None, &["restart", "sshd"])
}

// Gets sshd port number
//
// # Errors
//
// * fail to open ``/etc/ssh/sshd_config``
pub(crate) fn get() -> Result<u16> {
    let contents = fs::read_to_string(SSHD_CONFIG)?;
    let lines = contents.lines();

    for line in lines {
        if line.starts_with("Port ") {
            let s = line.split(' ').collect::<Vec<_>>();
            if let Some(port) = s.get(1) {
                if let Ok(port) = port.parse::<u16>() {
                    return Ok(port);
                }
            }
        }
    }
    Ok(SSHD_DEFAULT_PORT)
}
