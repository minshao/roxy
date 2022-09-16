pub mod common;
mod user;

use anyhow::{anyhow, Result};
use common::{NicOutput, Node, NodeRequest, SubCommand};
use serde::Deserialize;
use std::process::{Command, Stdio};

/// Returns usage of the partition mounted on `/data` using command `df -h`
/// as a tuple of mount point, total size, used size, and used rate.
///
/// # Errors
///
/// If `Regex` fails to compile a given regular expression,
/// then an error is returned.
pub fn disk_usage() -> Result<Option<(String, String, String, String)>> {
    user::hwinfo::disk_usage()
}

/// Returns a hostname.
///
/// # Errors
///
/// If `hostname::get` fails, then an error is returned.
pub fn hostname() -> Result<String> {
    if let Ok(host) = hostname::get() {
        Ok(host.to_string_lossy().to_string())
    } else {
        Err(anyhow!("Failed to get a hostname"))
    }
}

/// Returns how long the system has been running.
#[must_use]
pub fn uptime() -> Option<String> {
    user::hwinfo::uptime()
}

/// Returns a tuple of OS version and product version.
#[must_use]
pub fn version() -> (String, String) {
    user::hwinfo::get_version()
}

const FAIL_REQUEST: &str = "Failed to create a request";

/// Sets a version for OS.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If reading or writing of an OS version file fails, then an error
///   is returned.
pub fn set_os_version(ver: String) -> Result<String> {
    if let Ok(req) = NodeRequest::new::<String>(Node::Version(SubCommand::SetOsVersion), ver) {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Sets a version for product.
///
/// # Errors
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If reading or writing of a product version file fails, then an error
///   is returned.
pub fn set_product_version(ver: String) -> Result<String> {
    if let Ok(req) = NodeRequest::new::<String>(Node::Version(SubCommand::SetProductVersion), ver) {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Sets a hostname.
///
/// # Errors
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If `hostname::set` fails, then an error is returned.
pub fn set_hostname(host: String) -> Result<String> {
    if let Ok(req) = NodeRequest::new::<String>(Node::Hostname(SubCommand::Set), host) {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Returns tuples of (facilitiy, proto, addr) of syslog servers.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If it fails to open `/etc/rsyslog.d/50-default.conf`, then an error
///   is returned.
pub fn syslog_servers() -> Result<Option<Vec<(String, String, String)>>> {
    if let Ok(req) = NodeRequest::new::<Option<String>>(Node::Syslog(SubCommand::Get), None) {
        run_roxy::<Option<Vec<(String, String, String)>>>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Sets syslog servers.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If it fails to open or write `/etc/rsyslog.d/50-default.conf`, then
///   an error is returned.
/// * If it fails to restart rsyslogd service, then an error is returned.
pub fn set_syslog_servers(servers: Vec<String>) -> Result<String> {
    if let Ok(req) = NodeRequest::new::<Vec<String>>(Node::Syslog(SubCommand::Set), servers) {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Initiates syslog servers.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If it fails to open or write `/etc/rsyslog.d/50-default.conf`, then
///   an error is returned.
/// * If it fails to restart rsyslogd service, then an error is returned.
pub fn init_syslog_servers() -> Result<String> {
    if let Ok(req) = NodeRequest::new::<Option<String>>(Node::Syslog(SubCommand::Init), None) {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Returns the list of interface names.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
pub fn list_of_interfaces() -> Result<Vec<String>> {
    if let Ok(req) = NodeRequest::new::<Option<String>>(
        Node::Interface(SubCommand::List),
        Some(String::from("en")),
    ) {
        run_roxy::<Vec<String>>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Returns the setting of an interface.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
pub fn interface(dev: String) -> Result<Option<Vec<(String, NicOutput)>>> {
    if let Ok(req) = NodeRequest::new::<Option<String>>(Node::Interface(SubCommand::Get), Some(dev))
    {
        run_roxy::<Option<Vec<(String, NicOutput)>>>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Returns the settings of all the interfaces.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
pub fn interfaces() -> Result<Option<Vec<(String, NicOutput)>>> {
    if let Ok(req) = NodeRequest::new::<Option<String>>(Node::Interface(SubCommand::Get), None) {
        run_roxy::<Option<Vec<(String, NicOutput)>>>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Sets an interface setting.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If it fails to read or write a netplan yaml conf file, then an error
///   is returned.
/// * If dhcp4 and static ip address or nameserver address is set in the same
///   interface, then an error is returned.
/// * If a user tries to set a new gateway address when another interface has
///   the same, then an error is returned.
pub fn set_interface(
    dev: String,
    addresses: Option<Vec<String>>,
    dhcp4: Option<bool>,
    gateway4: Option<String>,
    nameservers: Option<Vec<String>>,
) -> Result<String> {
    let nic = NicOutput::new(addresses, dhcp4, gateway4, nameservers);
    if let Ok(req) =
        NodeRequest::new::<(String, NicOutput)>(Node::Interface(SubCommand::Set), (dev, nic))
    {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Reboots the system.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If `nix::sys::reboot::reboot` fails, then an error is returned.
pub fn reboot() -> Result<String> {
    if let Ok(req) = NodeRequest::new::<Option<String>>(Node::Reboot, None) {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Turns the system off.
///
/// # Errors
///
/// The following errors are possible:
///
/// * If serialization of command arguments does not succeed, then an error
///   is returned.
/// * If spawning the roxy executable fails, then an error is returned.
/// * If delivering a command to roxy fails, then an error is returned.
/// * If a response message from roxy is invalid regarding JSON syntax or
///   is not successfully base64-decoded, then an error is returned.
/// * If `nix::sys::reboot::reboot` fails, then an error is returned.
pub fn power_off() -> Result<String> {
    if let Ok(req) = NodeRequest::new::<Option<String>>(Node::PowerOff, None) {
        run_roxy::<String>(req)
    } else {
        Err(anyhow!(FAIL_REQUEST))
    }
}

/// Response message from Roxy to caller
#[derive(Deserialize, Debug)]
pub enum TaskResult {
    Ok(String),
    Err(String),
}

// TODO: fix the exact path to "roxy"
//
// # Errors
//
// * Failure to spawn roxy
// * Failure to write command to roxy
// * Invalid json syntax in response message
// * base64 decode error for reponse message
// * Received execution error from roxy
fn run_roxy<T>(req: NodeRequest) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let mut child = Command::new("roxy")
        .env(
            "PATH",
            "/usr/local/aice/bin:/usr/sbin:/usr/bin:/sbin:/bin:.",
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(child_stdin) = child.stdin.take() {
        std::thread::spawn(move || {
            serde_json::to_writer(child_stdin, &req).expect("`Task` should serialize to JSON");
        });
    } else {
        return Err(anyhow!("failed to execute roxy"));
    }

    let output = child.wait_with_output()?;
    match serde_json::from_reader::<&[u8], TaskResult>(&output.stdout) {
        Ok(TaskResult::Ok(x)) => {
            let decoded = base64::decode(&x).map_err(|_| anyhow!("fail to decode response."))?;
            Ok(bincode::deserialize::<T>(&decoded)?)
        }
        Ok(TaskResult::Err(x)) => Err(anyhow!("{}", x)),
        Err(e) => Err(anyhow!("fail to parse response. {}", e)),
    }
}
