// TODO: remove the below when proper codes are written
#![allow(dead_code)]
use super::{run_command, run_command_output};
use anyhow::{anyhow, Result};
use std::{
    net::{IpAddr, SocketAddr, TcpStream},
    thread,
    time::{Duration, SystemTime},
};

const AICE_SERVICES: [&str; 6] = ["zeek", "reconverge", "review", "hog", "peek", "reproduce"];
//const SYSTEM_SERVICES: [&str; 5] = ["rsyslogd", "ntp", "ufw", "postgres", "kafka"];

// Starts service
//
// # Errors
//
// * fail to execute command
pub(crate) fn start(service: &str) -> Result<bool> {
    run_command("systemctl", None, &["start", service])
}

// Stops service
//
// # Errors
//
// * fail to execute command
pub(crate) fn stop(service: &str) -> Result<bool> {
    run_command("systemctl", None, &["stop", service])
}

// Restarts service
//
// # Errors
//
// * fail to execute command
pub(crate) fn restart(service: &str) -> Result<bool> {
    run_command("systemctl", None, &["restart", service])
}

// # Errors
//
// * `systemctl` command not found
// * `cmd` is not one of `start`, `status`, `stop`
// * command execution error
fn service_control(service: &str, cmd: &str) -> Result<bool> {
    if !AICE_SERVICES.contains(&service) {
        return Err(anyhow!("Unknown service name"));
    }

    // match service {
    //     "zeek" | "reconverge" | "review" | "hog" | "peek" | "reproduce" => {}
    //     _ => return Err(anyhow!("Unknown service name")),
    // }

    run_command("systemctl", None, &[cmd, service])
}

#[must_use]
pub(crate) fn status(svc: Option<&str>) -> Vec<(String, String)> {
    let services = if let Some(s) = svc {
        vec![s]
    } else {
        AICE_SERVICES.to_vec()
    };
    let mut out = Vec::new();
    for &service in &services {
        let mut output = if service == "zeek" {
            run_command_output("systemctl", None, &["systemctl", "is-active", service])
        } else {
            run_command_output("systemctl", None, &["is-active", service])
        };
        if output.is_none() {
            output = run_command_output("systemctl", None, &["is-failed", service]);
        }

        if let Some(output) = output {
            out.push((service.to_string(), output));
        }
    }
    out
}

// # Errors
//
// * fail to stop all active services
pub(crate) fn stop_all() -> Result<()> {
    let st = status(None);
    for (service, state) in &st {
        if *state == "active" {
            service_control(service, "stop")?;
        }
    }
    Ok(())
}

// Check the port is open (service is available).
// * Be careful! The opened ports does not mean that service is available. Sometimes it takes more time.
// * The service running in docker container should wait more time until service is ready.
//
// # Errors
//
// * invalid ipaddress or port number
pub(crate) fn waitfor_up(addr: &str, port: &str, timeout: u64) -> Result<bool> {
    let remote_sock = SocketAddr::new(addr.parse::<IpAddr>()?, port.parse::<u16>()?);
    let start = SystemTime::now();
    loop {
        match TcpStream::connect_timeout(&remote_sock, Duration::from_secs(1)) {
            Ok(_) => return Ok(true),
            Err(_) => {
                if SystemTime::now().duration_since(start)?.as_secs() < timeout {
                    thread::sleep(Duration::from_secs(1));
                } else {
                    return Ok(false);
                }
            }
        }
    }
}