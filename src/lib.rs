#![no_std]
use heapless::String;
use no_std_net::Ipv4Addr;

#[macro_use]
extern crate bitflags;

mod codec;
mod ids;

/// Encapsulates errors that might occur when issuing or processing eRPCs.
#[derive(Debug, Clone, PartialEq)]
pub enum Err<E> {
    /// Parsing via the nom crate indicated an error
    Parsing(nom::Err<()>),
    /// The CRC was wrong
    CRCMismatch,
    /// There was an issue while transmitting
    TXErr,
    /// The response we were given to parse was for a different (callback,
    /// probably) RPC.
    NotOurs,
    /// There was an RPC-specific error.
    RPCErr(E),
    /// Too much data was present in the response
    ResponseOverrun,
    Unknown,
}

impl<E> From<nom::Err<()>> for Err<E> {
    fn from(nom_err: nom::Err<()>) -> Self {
        Err::Parsing::<E>(nom_err)
    }
}

pub use codec::{FrameHeader, Header};

/// Describes an RPC used by the system.
pub trait RPC {
    type ReturnValue;
    type Error;

    fn header(&self, seq: u32) -> Header;
    fn args(&self, _buff: &mut heapless::Vec<u8, 64>) {}

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>>;
}

mod system_rpcs;
mod tcpip_rpcs;
mod wifi_rpcs;

/// The RPCs which can be called to control the wifi.
pub mod rpcs {
    pub use crate::system_rpcs::*;
    pub use crate::tcpip_rpcs::*;
    pub use crate::wifi_rpcs::*;
}

/// Specifies a layer 3 interface to be affected by the command.
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum L3Interface {
    Station = 0,
    AP = 1,
}

/// Possible modes of the Wifi PHY.
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum WifiMode {
    None = 0,
    Station = 1,
    AP = 2,
    StationAndAP = 3,
    Promiscuous = 4,
    P2P = 5,
}

/// Describes the high-level type of a network.
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
#[repr(u32)]
pub enum BssType {
    Infra = 0,
    Adhoc = 1,
    Any = 2,
    Unknown = core::u32::MAX,
}

impl From<u32> for BssType {
    fn from(orig: u32) -> Self {
        match orig {
            0 => return BssType::Infra,
            1 => return BssType::Adhoc,
            2 => return BssType::Any,
            _ => return BssType::Unknown,
        };
    }
}

bitflags! {
    /// Specifies the security features of a network.
    pub struct Security: u32 {
        const WEP_ENABLED = 1;
        const TKIP_ENABLED = 2;
        const AES_ENABLED = 4;
        const AES_CMAC_ENABLED = 0x10;
        const SHARED_ENABLED = 0x00008000;
        const WPA_SECURITY = 0x00200000;
        const WPA2_SECURITY = 0x00400000;
        const WPA3_SECURITY = 0x00800000;
        const WPS_ENABLED = 0x10000000;
        const WEP_PSK = Self::WEP_ENABLED.bits;
        const WEP_SHARED = Self::WEP_ENABLED.bits | Self::SHARED_ENABLED.bits;
        const WPA_TKIP_PSK = Self::WPA_SECURITY.bits | Self::TKIP_ENABLED.bits;
        const WPA_AES_PSK = Self::WPA_SECURITY.bits | Self::AES_ENABLED.bits;
        const WPA2_AES_PSK  = Self::WPA2_SECURITY.bits | Self::AES_ENABLED.bits;
        const WPA2_TKIP_PSK = Self::WPA2_SECURITY.bits | Self::TKIP_ENABLED.bits;
        const WPA2_MIXED_PSK = Self::WPA2_SECURITY.bits | Self::AES_ENABLED.bits | Self::TKIP_ENABLED.bits;
        const WPA_WPA2_MIXED = Self::WPA_SECURITY.bits | Self::WPA2_SECURITY.bits;
        const WPA2_AES_CMAC = Self::WPA2_SECURITY.bits | Self::AES_CMAC_ENABLED.bits;
        const WPS_OPEN = Self::WPS_ENABLED.bits;
        const WPS_SECURE = Self::WPS_ENABLED.bits | Self::AES_ENABLED.bits;
        const WPS3_AES_PSK = Self::WPA3_SECURITY.bits | Self::AES_ENABLED.bits;
    }
}

/// Valid WPS modes.
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
#[repr(u32)]
pub enum WPS {
    Default = 0x0000,
    UserSpecifed = 0x0001,
    MachineSpecified = 0x0002,
    Rekey = 0x0003,
    Pushbutton = 0x0004,
    RegistrarSpecified = 0x0005,
    None = 0x0006,
    Wsc = 0x0007,
    Unknown = 0xffff,
}

impl From<u32> for WPS {
    fn from(orig: u32) -> Self {
        match orig {
            0 => return WPS::Default,
            1 => return WPS::UserSpecifed,
            2 => return WPS::MachineSpecified,
            3 => return WPS::Rekey,
            4 => return WPS::Pushbutton,
            5 => return WPS::RegistrarSpecified,
            6 => return WPS::None,
            7 => return WPS::Wsc,
            _ => return WPS::Unknown,
        };
    }
}

/// Valid wifi bands.
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(dead_code)]
#[repr(u32)]
pub enum Band {
    _5Ghz = 0,
    _24Ghz = 1,
    Unknown = 0xffff,
}

impl From<u32> for Band {
    fn from(orig: u32) -> Self {
        match orig {
            0 => return Band::_5Ghz,
            1 => return Band::_24Ghz,
            _ => return Band::Unknown,
        };
    }
}

/// The machine-readable network name (6-bytes).
#[derive(Copy, Clone)]
#[repr(packed)]
pub struct BSSID(pub [u8; 6]);

impl core::fmt::Debug for BSSID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let table = b"0123456789abcdef";

        let mut out = [0u8; 12 + 6 - 1];
        for i in 0..(12 + 6 - 1) {
            let b = self.0[i / 3];
            out[i] = match (i + 1) % 3 {
                0 => ':' as u8,
                1 => table[(b >> 4) as usize],
                2 => table[(b & 0xf) as usize],
                _ => '?' as u8,
            }
        }

        f.write_str(core::str::from_utf8(&out).unwrap())
    }
}

/// A human-readable network name.
#[derive(Copy, Clone)]
#[repr(packed)]
pub struct SSID {
    len: u8,
    value: [u8; 33],
}

impl core::fmt::Debug for SSID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Unused unsafe warning is erroneous: needed for safe_packed_borrows
        #[allow(unused_unsafe)]
        unsafe {
            f.write_str(core::str::from_utf8(&self.value[..self.len as usize]).unwrap())
        }
    }
}

impl<const N: usize> Into<String<N>> for SSID {
    fn into(self) -> String<N> {
        let mut out = String::new();
        // Unused unsafe warning is erroneous: needed for safe_packed_borrows
        #[allow(unused_unsafe)]
        unsafe {
            for i in 0..self.len as usize {
                out.push(self.value[i] as char).ok();
            }
        }
        out
    }
}

/// Describes layer 3 (IP) configuration.
#[derive(Debug, Clone)]
pub struct IPInfo {
    pub ip: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub gateway: Ipv4Addr,
}
