//! Network interface information.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// A network interface on the system.
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// Interface name (e.g., "eth0", "en0", "Wi-Fi").
    pub name: String,
    /// Human-readable description (Windows only, empty on other platforms).
    pub description: String,
    /// MAC address, if available.
    pub mac_address: Option<MacAddress>,
    /// IPv4 addresses assigned to this interface.
    pub ipv4_addresses: Vec<Ipv4Info>,
    /// IPv6 addresses assigned to this interface.
    pub ipv6_addresses: Vec<Ipv6Info>,
    /// Interface type (Ethernet, WiFi, Loopback, etc.).
    pub interface_type: InterfaceType,
    /// Whether the interface is currently up.
    pub is_up: bool,
    /// Maximum transmission unit in bytes.
    pub mtu: Option<u32>,
    /// Interface index.
    pub index: u32,
}

/// MAC (hardware) address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// Create a new MAC address from bytes.
    pub fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes.
    pub fn octets(&self) -> [u8; 6] {
        self.0
    }
}

impl std::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// IPv4 address information.
#[derive(Debug, Clone)]
pub struct Ipv4Info {
    /// The IPv4 address.
    pub address: Ipv4Addr,
    /// Network prefix length (CIDR notation).
    pub prefix_len: u8,
    /// Netmask derived from prefix length.
    pub netmask: Ipv4Addr,
}

impl Ipv4Info {
    fn prefix_to_netmask(prefix_len: u8) -> Ipv4Addr {
        if prefix_len >= 32 {
            Ipv4Addr::new(255, 255, 255, 255)
        } else if prefix_len == 0 {
            Ipv4Addr::new(0, 0, 0, 0)
        } else {
            let mask = !((1u32 << (32 - prefix_len)) - 1);
            Ipv4Addr::from(mask.to_be_bytes())
        }
    }
}

/// IPv6 address information.
#[derive(Debug, Clone)]
pub struct Ipv6Info {
    /// The IPv6 address.
    pub address: Ipv6Addr,
    /// Network prefix length.
    pub prefix_len: u8,
}

/// Type of network interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceType {
    /// Ethernet interface.
    Ethernet,
    /// Wireless (WiFi) interface.
    WiFi,
    /// Loopback interface (localhost).
    Loopback,
    /// Virtual or tunnel interface.
    Virtual,
    /// Unknown interface type.
    Unknown,
}

impl std::fmt::Display for InterfaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceType::Ethernet => write!(f, "Ethernet"),
            InterfaceType::WiFi => write!(f, "WiFi"),
            InterfaceType::Loopback => write!(f, "Loopback"),
            InterfaceType::Virtual => write!(f, "Virtual"),
            InterfaceType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl NetworkInterface {
    /// Get all network interfaces on the system.
    pub fn list() -> Vec<NetworkInterface> {
        let interfaces = match netdev::get_interfaces() {
            ifaces => ifaces,
        };

        interfaces
            .into_iter()
            .map(|iface| {
                let mac_address = iface.mac_addr.map(|mac| MacAddress::new(mac.octets()));

                let ipv4_addresses: Vec<Ipv4Info> = iface
                    .ipv4
                    .iter()
                    .map(|net| Ipv4Info {
                        address: net.addr(),
                        prefix_len: net.prefix_len(),
                        netmask: Ipv4Info::prefix_to_netmask(net.prefix_len()),
                    })
                    .collect();

                let ipv6_addresses: Vec<Ipv6Info> = iface
                    .ipv6
                    .iter()
                    .map(|net| Ipv6Info {
                        address: net.addr(),
                        prefix_len: net.prefix_len(),
                    })
                    .collect();

                let interface_type = if iface.is_loopback() {
                    InterfaceType::Loopback
                } else if iface.is_tun() {
                    InterfaceType::Virtual
                } else {
                    // netdev doesn't distinguish WiFi from Ethernet directly
                    // On macOS, "en0" is typically WiFi, but this isn't reliable
                    InterfaceType::Ethernet
                };

                NetworkInterface {
                    name: iface.name.clone(),
                    description: iface.description.clone().unwrap_or_default(),
                    mac_address,
                    ipv4_addresses,
                    ipv6_addresses,
                    interface_type,
                    is_up: iface.is_up(),
                    mtu: None, // MTU not available in netdev crate
                    index: iface.index,
                }
            })
            .collect()
    }

    /// Get the default network interface (used for internet traffic).
    pub fn default_interface() -> Option<NetworkInterface> {
        netdev::get_default_interface().ok().map(|iface| {
            let mac_address = iface.mac_addr.map(|mac| MacAddress::new(mac.octets()));

            let ipv4_addresses: Vec<Ipv4Info> = iface
                .ipv4
                .iter()
                .map(|net| Ipv4Info {
                    address: net.addr(),
                    prefix_len: net.prefix_len(),
                    netmask: Ipv4Info::prefix_to_netmask(net.prefix_len()),
                })
                .collect();

            let ipv6_addresses: Vec<Ipv6Info> = iface
                .ipv6
                .iter()
                .map(|net| Ipv6Info {
                    address: net.addr(),
                    prefix_len: net.prefix_len(),
                })
                .collect();

            let interface_type = if iface.is_loopback() {
                InterfaceType::Loopback
            } else if iface.is_tun() {
                InterfaceType::Virtual
            } else {
                InterfaceType::Ethernet
            };

            NetworkInterface {
                name: iface.name.clone(),
                description: iface.description.clone().unwrap_or_default(),
                mac_address,
                ipv4_addresses,
                ipv6_addresses,
                interface_type,
                is_up: iface.is_up(),
                mtu: None, // MTU not available in netdev crate
                index: iface.index,
            }
        })
    }

    /// Get the default gateway information.
    pub fn default_gateway() -> Option<GatewayInfo> {
        netdev::get_default_gateway().ok().map(|gw| {
            // Get the first IPv4 address, or fall back to first IPv6
            // The gateway's ipv4/ipv6 fields are Vec<Ipv4Addr>/Vec<Ipv6Addr>
            let ip_address: IpAddr = gw
                .ipv4
                .first()
                .map(|addr| IpAddr::V4(*addr))
                .or_else(|| gw.ipv6.first().map(|addr| IpAddr::V6(*addr)))
                .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

            let mac_address = Some(MacAddress::new(gw.mac_addr.octets()));

            GatewayInfo {
                ip_address,
                mac_address,
            }
        })
    }

    /// Get all IP addresses (both v4 and v6) for this interface.
    pub fn all_addresses(&self) -> Vec<IpAddr> {
        let mut addrs: Vec<IpAddr> = self
            .ipv4_addresses
            .iter()
            .map(|info| IpAddr::V4(info.address))
            .collect();
        addrs.extend(
            self.ipv6_addresses
                .iter()
                .map(|info| IpAddr::V6(info.address)),
        );
        addrs
    }

    /// Check if this interface has any IP addresses assigned.
    pub fn has_addresses(&self) -> bool {
        !self.ipv4_addresses.is_empty() || !self.ipv6_addresses.is_empty()
    }

    /// Check if this is the loopback interface.
    pub fn is_loopback(&self) -> bool {
        self.interface_type == InterfaceType::Loopback
    }
}

/// Default gateway information.
#[derive(Debug, Clone)]
pub struct GatewayInfo {
    /// Gateway IP address.
    pub ip_address: IpAddr,
    /// Gateway MAC address, if available.
    pub mac_address: Option<MacAddress>,
}
