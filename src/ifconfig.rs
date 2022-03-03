use crate::{list_files, run_command};
use anyhow::{anyhow, Result};
use ipnet::IpNet;
use pnet::datalink::interfaces;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::net::IpAddr;
use std::{
    collections::HashMap,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
};

const NETPLAN_PATH: &str = "/etc/netplan";
const DEFAULT_NETPLAN_YAML: &str = "01-netcfg.yaml";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Nic {
    #[serde(skip_serializing_if = "Option::is_none")]
    addresses: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dhcp4: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gateway4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nameservers: Option<HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optional: Option<bool>,
}

impl fmt::Display for Nic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(s) = serde_yaml::to_string(self) {
            write!(f, "{}", s)
        } else {
            Ok(())
        }
    }
}

impl Nic {
    #[must_use]
    pub fn new(
        addresses: Option<Vec<String>>,
        dhcp4: Option<bool>,
        gateway4: Option<String>,
        nameservers: Option<HashMap<String, Vec<String>>>,
        optional: Option<bool>,
    ) -> Self {
        Nic {
            addresses,
            dhcp4,
            gateway4,
            nameservers,
            optional,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NicOutput {
    addresses: Option<Vec<String>>,
    dhcp4: Option<bool>,
    gateway4: Option<String>,
    nameservers: Option<Vec<String>>,
}

impl NicOutput {
    #[must_use]
    pub fn new(
        addresses: Option<Vec<String>>,
        dhcp4: Option<bool>,
        gateway4: Option<String>,
        nameservers: Option<Vec<String>>,
    ) -> Self {
        NicOutput {
            addresses,
            dhcp4,
            gateway4,
            nameservers,
        }
    }

    #[must_use]
    pub fn to(&self) -> Nic {
        let nameservers = if let Some(nm) = &self.nameservers {
            let mut m = HashMap::new();
            m.insert("addresses".to_string(), nm.clone());
            m.insert("search".to_string(), Vec::new());
            Some(m)
        } else {
            None
        };
        Nic {
            addresses: self.addresses.clone(),
            dhcp4: self.dhcp4,
            gateway4: self.gateway4.clone(),
            nameservers,
            optional: None,
        }
    }

    #[must_use]
    pub fn from(nic: &Nic) -> Self {
        let nameservers = {
            if let Some(nm) = &nic.nameservers {
                nm.get("addresses").cloned()
            } else {
                None
            }
        };
        NicOutput {
            addresses: nic.addresses.clone(),
            dhcp4: nic.dhcp4,
            gateway4: nic.gateway4.clone(),
            nameservers,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<Vec<String>>,
    addresses: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Bridge {
    interfaces: Vec<String>,
    addresses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gateway4: Option<String>,
    nameservers: Address,
}

// only support ethernets, bridges. No wifis support.
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct Network {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    renderer: Option<String>,
    #[serde_as(as = "HashMap<_, _>")]
    ethernets: Vec<(String, Nic)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bridges: Option<HashMap<String, Bridge>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetplanYaml {
    network: Network,
}

impl fmt::Display for NetplanYaml {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(s) = serde_yaml::to_string(self) {
            write!(f, "{}", s)
        } else {
            Ok(())
        }
    }
}

impl NetplanYaml {
    /// # Errors
    /// * fail to open netplan yaml file
    /// * fail to read yaml file
    /// * fail to parse yaml file
    pub fn new(path: &str) -> Result<Self> {
        let mut f = File::open(path)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        match serde_yaml::from_str::<NetplanYaml>(&buf) {
            Ok(r) => Ok(r),
            Err(e) => Err(anyhow!("Error: {}", e)),
        }
    }

    /// merge two yaml conf into one
    /// The merged conf will applied to system when save() is called.
    pub fn merge(&mut self, newyml: Self) {
        if newyml.network.version.is_some() {
            self.network.version = newyml.network.version;
        }
        if newyml.network.renderer.is_some() {
            self.network.renderer = newyml.network.renderer;
        }
        for (ifname, ifcfg) in newyml.network.ethernets {
            if let Some(item) = self.network.ethernets.iter_mut().find(|x| x.0 == ifname) {
                item.1 = ifcfg;
            } else {
                self.network.ethernets.push((ifname, ifcfg));
            }
        }
        self.network.ethernets.sort_by(|a, b| a.0.cmp(&b.0));

        if let Some(new_bridges) = newyml.network.bridges {
            if let Some(self_bridges) = &mut self.network.bridges {
                for (ifname, bridgecfg) in new_bridges {
                    if let Some(item) = self_bridges.get_mut(&ifname) {
                        *item = bridgecfg;
                    } else {
                        self_bridges.insert(ifname, bridgecfg);
                    }
                }
            }
        }
    }

    /// apply() should be run to apply this change.
    pub fn set_interface(&mut self, ifname: &str, new_if: Nic) {
        if let Some(item) = self.network.ethernets.iter_mut().find(|x| x.0 == *ifname) {
            item.1 = new_if;
        } else {
            self.network.ethernets.push((ifname.to_string(), new_if));
            self.network.ethernets.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    /// apply() should be run to apply this change.
    pub fn init_interface(&mut self, ifname: &str) {
        let new_if = Nic::new(None, None, None, None, None);
        Self::set_interface(self, ifname, new_if);
    }

    /// Remove interface address, gateway4, nameservers.
    /// apply() should be run to apply this change.
    ///
    /// # Recommendation:
    /// * use use set() command instead of delete() if possible
    ///
    /// # Errors
    /// * interface not found
    pub fn delete(&mut self, ifname: &str, nic_output: &NicOutput) -> Result<()> {
        let ifs = if let Some((_, ifs)) = self
            .network
            .ethernets
            .iter_mut()
            .find(|(name, _)| *name == *ifname)
        {
            ifs
        } else {
            return Err(anyhow!("interface not found!"));
        };

        if let Some(addrs) = &nic_output.addresses {
            for addr in addrs {
                if let Some(ifs_addrs) = &mut ifs.addresses {
                    ifs_addrs.retain(|x| *x != *addr);
                }
            }
        }

        if nic_output.gateway4.is_some() && ifs.gateway4 == nic_output.gateway4 {
            ifs.gateway4 = None;
        }

        if let Some(addrs) = &nic_output.nameservers {
            for addr in addrs {
                if let Some(ifs_nameservers) = &mut ifs.nameservers {
                    for v in ifs_nameservers.values_mut() {
                        if v.contains(addr) {
                            v.retain(|x| *x != *addr);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // TODO: synchronize /etc/netplan/--yaml vs nic running conf
    // pub fn sync(&self, _dir: &str) -> usize {
    //     0
    // }

    /// save conf to netplan yaml file, and apply it to system.
    /// merge all yaml files under /etc/netplan folder
    /// # Errors
    /// * fail to get /etc/netplan yaml files
    /// * fail to create or write temporary yaml file in /tmp
    /// * fail to copy yaml file from /tmp to /etc/netplan
    /// * fail to remove temporary file
    /// * fail to remove /etc/netplan files except the first yaml file
    /// * fail to run netplan apply command
    pub fn apply(&self, dir: &str) -> Result<()> {
        let files = match list_files(dir, None, false) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        let mut from = format!("/tmp/{}", DEFAULT_NETPLAN_YAML);
        let mut to = format!("{dir}/{}", DEFAULT_NETPLAN_YAML);
        if let Some((_, _, first)) = files.first() {
            if first != DEFAULT_NETPLAN_YAML {
                from = format!("/tmp/{first}");
                to = format!("{dir}/{first}");
            }
        }

        let mut tmp = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&from)?;
        write!(tmp, "{}", self)?;

        fs::copy(&from, &to)?;
        fs::remove_file(&from)?;

        for (_, _, file) in &files {
            let path = format!("{dir}/{}", file);
            if path != to {
                fs::remove_file(&path)?;
            }
        }

        run_command("netplan", None, &["apply"])?;
        Ok(())
    }
}

/// get all interface settings
/// get all netplan yaml conf from /etc/netplan and merge it into one.
/// # Errors
/// * fail to get yaml files from the /etc/netplan
/// * fail to parse yaml file
/// * yaml file not found
fn load_netplan_yaml(dir: &str) -> Result<NetplanYaml> {
    let files = list_files(dir, None, false)?;
    let mut netplan: Option<NetplanYaml> = None;
    for (_, _, file) in files {
        let path = format!("{}/{}", dir, file);
        let netplan_cfg = NetplanYaml::new(&path)?;
        if let Some(n) = &mut netplan {
            n.merge(netplan_cfg);
        } else {
            netplan = Some(netplan_cfg);
        }
    }
    if let Some(n) = netplan {
        Ok(n)
    } else {
        Err(anyhow!("Netplan configuration not found!"))
    }
}

/// Validate ipv4/ipv6 networks
/// # Errors
/// * invalid ip network format
fn validate_ipnetworks(ipnetwork: &str) -> Result<()> {
    match ipnetwork.parse::<IpNet>() {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("{:?}", e)),
    }
}

/// Validate ipv4, ipv6 address
/// # Errors
/// * invalid ip address format
fn validate_ipaddress(ipaddr: &str) -> Result<()> {
    match ipaddr.parse::<IpAddr>() {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("{:?}", e)),
    }
}

/// Initialize interface.
///
/// Be careful!. Netplan may remove address only in the yaml file.
/// The addresess cab be remained in the running interface after netplan apply.
/// To avoid this case, this function execute ifconfig system command internally.
///
/// # Errors
/// * interface name not found
/// * fail to load /etc/netplan yaml files
/// * fail to execute netplan apply
/// * fail to ifconfig command
pub fn init(ifname: &str) -> Result<()> {
    let mut netplan = load_netplan_yaml(NETPLAN_PATH)?;
    let all_interfaces = interfaces();
    for iface in all_interfaces {
        if iface.name == *ifname {
            netplan.init_interface(ifname);
            netplan.apply(NETPLAN_PATH)?;

            // init running interface setting with ifconfig command
            // because 'netplan apply' command would not init the running settings.
            run_command("ifconfig", None, &[ifname, "0.0.0.0"])?;
            run_command("ifconfig", None, &[ifname, "up"])?;

            return Ok(());
        }
    }

    Err(anyhow!("interface \"{}\" not found.", ifname))
}

/// Set interface ip address or gateway address or nameservers.
/// This command will OVERWRITE all existing setting in the interface if exist.
///
/// # Warning
/// * if the target interface is not running (cable connected), netplan does not
///   set the address to interface. Instead it will just saved it into conf file.
///
/// # Example
/// ```
/// // To replace(overwrite) ip address, gateway, nameservers of eno3 interface.
/// let nic_output = NicOutput::new(
///     Some(vec!["192.168.0.205/24".to_string(), "192.168.4.7/24".to_string()]),
///     None,
///     Some("192.168.0.1".to_string()),
///     Some(vec!["164.124.101.1".to_string(), "164.124.101.2".to_string()])
/// );
/// ifconfig::set("eno3", &nic_output)?;
/// ```
/// # Errors
/// * fail to get or save, apply netplan yaml conf
/// * dhcp4 and static ip address or nameserver address is set in same interface
/// * try to set new gateway address when other interface already have the gateway
pub fn set(ifname: &str, nic_output: &NicOutput) -> Result<()> {
    let mut netplan = load_netplan_yaml(NETPLAN_PATH)?;

    if let Some(addrs) = &nic_output.addresses {
        for ipnetwork in addrs {
            if let Err(e) = validate_ipnetworks(ipnetwork) {
                return Err(anyhow!("invalid interface address: {}. {:?}", ipnetwork, e));
            }
        }
    }

    if let Some(ipaddr) = &nic_output.gateway4 {
        if let Err(e) = validate_ipaddress(ipaddr) {
            return Err(anyhow!("invalid gateway4 address: {}. {:?}", ipaddr, e));
        }

        for (nic_name, nic) in &netplan.network.ethernets {
            if nic_name != ifname && nic.gateway4.is_some() {
                return Err(anyhow!("only one interface can have gateway."));
            }
        }
    }

    for ip in &nic_output.nameservers {
        for ipaddr in ip {
            if let Err(e) = validate_ipaddress(ipaddr) {
                return Err(anyhow!("invalid nameserver address: {}. {:?}", ipaddr, e));
            }
        }
    }

    if nic_output.dhcp4 == Some(true)
        && (nic_output.addresses.is_some() || nic_output.nameservers.is_some())
    {
        return Err(anyhow!(
            "dhcp4 and static address cannot be set in the same interface"
        ));
    }

    netplan.set_interface(ifname, nic_output.to());
    netplan.apply(NETPLAN_PATH)?;
    Ok(())
}

/// Get interface configurations
/// # Example
/// ```
/// // get all interfaces
/// let all_interfaces = ifconfig::get(&None)?;
///
/// // get "eno1" interface
/// let eno1_interface = ifconfig::get(&Some("eno1".to_string()))?;
/// ```
/// # Errors
/// * fail to load /etc/netplan yaml files
pub fn get(ifname: &Option<String>) -> Result<Option<Vec<(String, NicOutput)>>> {
    let netplan = load_netplan_yaml(NETPLAN_PATH)?;
    if let Some(name) = ifname {
        if let Some((_, nic)) = netplan.network.ethernets.iter().find(|(x, _)| *x == *name) {
            return Ok(Some(vec![(name.to_string(), NicOutput::from(nic))]));
        }
    } else {
        let mut nic_output = Vec::new();
        for (name, nic) in &netplan.network.ethernets {
            nic_output.push((name.to_string(), NicOutput::from(nic)));
        }
        return Ok(Some(nic_output));
    }
    Ok(None)
}

/// Remove interface or name server or gateway address from the specified interface.
///
/// # Example
///```
/// // to delete interface address "192.168.3.7/24", nameserver "164.124.101.2"
/// let nic_output = NicOutput::new(
///     Some(vec!["192.168.3.7/24".to_string()]),
///     None,
///     None,
///     Some(vec!["164.124.101.2".to_string()]),);
///
/// ifconfig::delete("eno3", &nic_output)?;
///```
/// # Errors
/// * fail to load /etc/netplan yaml files
/// * fail to apply the change to system
/// * interface not found
pub fn delete(ifname: &str, nic_output: &NicOutput) -> Result<()> {
    let mut netplan = load_netplan_yaml(NETPLAN_PATH)?;
    netplan.delete(ifname, nic_output)?;
    netplan.apply(NETPLAN_PATH)?;

    if let Some(addrs) = &nic_output.addresses {
        for addr in addrs {
            // apply to running interface
            // if the device does not have this ip address, then this command will return ERROR!!!!
            run_command("ip", None, &["addr", "del", addr, "dev", ifname])?;
        }
    }
    Ok(())
}

/// Get interface names starting with the specified prefix
/// # Example
/// ```
/// // get interface names starting with "en"
/// let names = ifconfig::get_interface_names(&Some("en".to_string()));
/// ```
#[must_use]
pub fn get_interface_names(arg: &Option<String>) -> Vec<String> {
    let mut nics = interfaces();
    if let Some(prefix) = arg {
        nics.retain(|f| f.name.starts_with(prefix));
    }
    nics.iter().map(|f| f.name.clone()).collect()
}
