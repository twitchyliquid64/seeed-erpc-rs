#[allow(dead_code)]
use super::{codec, ids, Err};
use generic_array::{ArrayLength, GenericArray};
use heapless::{consts::U18, String};
use nom::{
    lib::std::ops::RangeFrom, lib::std::ops::RangeTo, number::streaming, InputIter, InputLength,
    Slice,
};

/// Returns the mac address as a colon-separated hex string.
pub struct GetMacAddress {}

impl super::RPC for GetMacAddress {
    type ReturnValue = String<U18>;
    type Error = i32;

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::GetMacAddress.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::GetMacAddress.into()
        {
            return Err(Err::NotOurs);
        }

        if data.input_len() < 18 {
            return Err(Err::RPCErr(-1));
        }
        let mut mac: String<U18> = String::new();
        for b in data.slice(RangeTo { end: 17 }).iter_elements() {
            mac.push(b as char).map_err(|_| Err::ResponseOverrun)?;
        }

        let (_, result) = streaming::le_u32(data.slice(RangeFrom { start: 18 }))?;
        if result != 0 {
            Err(Err::RPCErr(result as i32))
        } else {
            Ok(mac)
        }
    }
}

/// Returns true if the wifi chip is currently scanning.
pub struct IsScanning {}

impl super::RPC for IsScanning {
    type ReturnValue = bool;
    type Error = ();

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::IsScanning.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::IsScanning.into()
        {
            return Err(Err::NotOurs);
        }

        if data.input_len() < 1 {
            return Err(Err::RPCErr(()));
        }
        Ok(data.iter_elements().nth(0) != Some(0))
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
#[repr(u32)]
pub enum BssType {
    Infra = 0,
    Adhoc = 1,
    Any = 2,
    Unknown = core::u32::MAX,
}

bitflags! {
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
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(dead_code)]
#[repr(u32)]
pub enum Band {
    _5Ghz = 0,
    _24Ghz = 1,
}

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

impl<N> Into<String<N>> for SSID
where
    N: heapless::ArrayLength<u8>,
{
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

/// Describes a wifi network or station discovered via scanning.
#[derive(Copy, Clone)]
#[repr(packed)]
pub struct ScanResult {
    /// Service Set Identification (i.e. Name of Access Point)
    pub ssid: SSID,
    /// Basic Service Set Identification (i.e. MAC address of Access Point)
    pub bssid: BSSID,
    /// Receive Signal Strength Indication in dBm. <-90=poor, >-30=Excellent
    pub rssi: i16,
    /// Network type
    pub bss_type: BssType,
    /// Security type
    pub security: Security,
    /// WPS type
    pub wps: WPS,
    /// Channel
    pub chan: u32,
    /// Radio channel that the AP beacon was received on
    pub band: Band,
}

impl core::fmt::Debug for ScanResult {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Unused unsafe warning is erroneous: needed for safe_packed_borrows
        #[allow(unused_unsafe)]
        unsafe {
            if self.ssid.len > 0 {
                f.debug_struct("ScanResult")
                    .field("ssid", &self.ssid)
                    .field("bssid", &self.bssid)
                    .field("rssi", &self.rssi)
                    .field("type", &self.bss_type)
                    .field("security", &self.security)
                    .field("wps", &self.wps)
                    .field("channel", &self.chan)
                    .field("band", &self.band)
                    .finish()
            } else {
                f.debug_struct("ScanResult")
                    .field("bssid", &self.bssid)
                    .field("rssi", &self.rssi)
                    .field("type", &self.bss_type)
                    .field("security", &self.security)
                    .field("wps", &self.wps)
                    .field("channel", &self.chan)
                    .field("band", &self.band)
                    .finish()
            }
        }
    }
}

impl Default for ScanResult {
    fn default() -> Self {
        Self {
            ssid: SSID {
                len: 0,
                value: [0u8; 33],
            },
            bssid: BSSID([0u8; 6]),
            rssi: 0,
            bss_type: BssType::Any,
            security: Security::empty(),
            wps: WPS::Default,
            chan: 0,
            band: Band::_24Ghz,
        }
    }
}

/// Returns N number of scan results. This RPC must only be called after starting a
/// scan, and after IsScanning returns false.
pub struct ScanGetAP<N: ArrayLength<ScanResult>> {
    m: core::marker::PhantomData<N>,
}

impl<N: ArrayLength<ScanResult>> ScanGetAP<N> {
    pub fn new() -> Self {
        Self {
            m: core::marker::PhantomData,
        }
    }
}

impl<N: ArrayLength<ScanResult>> super::RPC for ScanGetAP<N> {
    type ReturnValue = (GenericArray<ScanResult, N>, i32);
    type Error = usize;

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::ScanGetAP.into(),
        }
    }

    fn args(&self, buff: &mut heapless::Vec<u8, heapless::consts::U64>) {
        let num = N::to_u16().to_le_bytes();
        buff.extend_from_slice(&num).ok();
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::ScanGetAP.into()
        {
            return Err(Err::NotOurs);
        }

        let (data, l) = streaming::le_u32(data)?; // Binary len - returning 62 bytes per result
        if l as usize != (core::mem::size_of::<ScanResult>() * N::to_usize()) {
            return Err(Err::ResponseOverrun);
        }

        let mut res = GenericArray::<ScanResult, N>::default();

        //let (data, ssid) = SSids::parse(data)?;
        for i in 0..N::to_usize() {
            res[i] = unsafe { *((data.as_ptr() as *const ScanResult).offset(i as isize)) };
        }

        let (_, ret_val) = streaming::le_i32(data.slice(core::mem::size_of::<ScanResult>()..))?;
        Ok((res, ret_val))
    }
}

/// Returns the number of APs which were detected.
pub struct ScanGetNumAPs {}

impl super::RPC for ScanGetNumAPs {
    type ReturnValue = u16;
    type Error = ();

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::ScanGetNumAPs.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::ScanGetNumAPs.into()
        {
            return Err(Err::NotOurs);
        }

        if data.input_len() < 2 {
            return Err(Err::RPCErr(()));
        }
        let (_, num) = streaming::le_u16(data)?;
        Ok(num)
    }
}

/// Initiates a network scan. A return value of 0 indicates success afaict.
pub struct ScanStart {}

impl super::RPC for ScanStart {
    type ReturnValue = i32;
    type Error = ();

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::ScanStart.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::ScanStart.into()
        {
            return Err(Err::NotOurs);
        }

        let (_, num) = streaming::le_i32(data)?;
        Ok(num)
    }
}
