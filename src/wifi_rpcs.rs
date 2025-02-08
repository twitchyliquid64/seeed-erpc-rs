#[allow(dead_code)]
use super::{codec, ids, Err};
use generic_array::{ArrayLength, GenericArray};
use heapless::String;
use nom::{
    bytes::streaming::take, lib::std::ops::RangeFrom, lib::std::ops::RangeTo, number::streaming,
    InputIter, InputLength, Slice,
};

/// Returns the mac address as a colon-separated hex string.
pub struct GetMacAddress {}

impl super::RPC for GetMacAddress {
    type ReturnValue = String<18>;
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
        let mut mac: String<18> = String::new();
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

/// Describes a wifi network or station discovered via scanning.
#[derive(Copy, Clone)]
pub struct ScanResult {
    /// Service Set Identification (i.e. Name of Access Point)
    pub ssid: super::SSID,
    /// Basic Service Set Identification (i.e. MAC address of Access Point)
    pub bssid: super::BSSID,
    /// Receive Signal Strength Indication in dBm. <-90=poor, >-30=Excellent
    pub rssi: i16,
    /// Network type
    pub bss_type: super::BssType,
    /// Security type
    pub security: super::Security,
    /// WPS type
    pub wps: super::WPS,
    /// Channel
    pub chan: u32,
    /// Radio channel that the AP beacon was received on
    pub band: super::Band,
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
            ssid: super::SSID {
                len: 0,
                value: [0u8; 33],
            },
            bssid: super::BSSID([0u8; 6]),
            rssi: 0,
            bss_type: super::BssType::Any,
            security: super::Security::empty(),
            wps: super::WPS::Default,
            chan: 0,
            band: super::Band::_24Ghz,
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

    fn args(&self, buff: &mut heapless::Vec<u8, 64>) {
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

        let (mut data, l) = streaming::le_u32(data)?; // Binary len - returning 62 bytes per result
        if l as usize != (62 * N::to_usize()) {
            return Err(Err::ResponseOverrun);
        }

        use core::convert::TryInto;
        let mut res = GenericArray::<ScanResult, N>::default();
        for i in 0..N::to_usize() {
            let (d, ssid_len) = streaming::le_u8(data)?;
            let (d, ssid_data) = take(33usize)(d)?;
            let (d, bssid) = take(6usize)(d)?;
            let (d, rssi) = streaming::le_i16(d)?;
            let (d, bss_type) = streaming::le_u32(d)?;
            let (d, security) = streaming::le_u32(d)?;
            let (d, wps) = streaming::le_u32(d)?;
            let (d, chan) = streaming::le_u32(d)?;
            let (d, band) = streaming::le_u32(d)?;

            res[i] = ScanResult {
                ssid: super::SSID {
                    len: ssid_len,
                    value: ssid_data.try_into().unwrap(),
                },
                bssid: super::BSSID(bssid.try_into().unwrap()),
                rssi,
                bss_type: bss_type.into(),
                security: super::Security::from_bits_truncate(security),
                wps: wps.into(),
                chan,
                band: band.into(),
            };
            data = d;
        }

        let (_, ret_val) = streaming::le_i32(data)?;
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

/// Turns on Wifi.
pub struct WifiOn {
    pub mode: super::WifiMode,
}

impl super::RPC for WifiOn {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 64>) {
        let mode = self.mode as u32;
        buff.extend_from_slice(&mode.to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::TurnOn.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::TurnOn.into()
        {
            return Err(Err::NotOurs);
        }

        let (_, num) = streaming::le_i32(data)?;
        Ok(num)
    }
}

/// Turns off Wifi.
pub struct WifiOff {}

impl super::RPC for WifiOff {
    type ReturnValue = i32;
    type Error = ();

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::TurnOff.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::TurnOff.into()
        {
            return Err(Err::NotOurs);
        }

        let (_, num) = streaming::le_i32(data)?;
        Ok(num)
    }
}

/// Connects to the network with the provided properties.
pub struct WifiConnect {
    pub ssid: String<64>,
    pub password: String<64>,
    pub security: super::Security,
    //key_id: u32,
    pub semaphore: u32,
}

impl super::RPC for WifiConnect {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 64>) {
        buff.extend_from_slice(&(self.ssid.len() as u32).to_le_bytes())
            .ok();
        buff.extend_from_slice(self.ssid.as_ref()).ok();

        // Write the nullable flag (0 = NotNull, 1 = Null)
        buff.push(if self.password.len() > 0 { 0u8 } else { 1u8 })
            .ok();
        if self.password.len() > 0 {
            buff.extend_from_slice(&(self.password.len() as u32).to_le_bytes())
                .ok();
            buff.extend_from_slice(self.password.as_ref()).ok();
        }

        buff.extend_from_slice(&(self.security.bits()).to_le_bytes())
            .ok();
        buff.extend_from_slice(&(0u32.wrapping_sub(1)).to_le_bytes())
            .ok(); // key_id - always -1?
        buff.extend_from_slice(&(self.semaphore).to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::Wifi,
            request: ids::WifiRequest::Connect.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::Wifi
            || hdr.request != ids::WifiRequest::Connect.into()
        {
            return Err(Err::NotOurs);
        }

        let (_, num) = streaming::le_i32(data)?;
        Ok(num)
    }
}
