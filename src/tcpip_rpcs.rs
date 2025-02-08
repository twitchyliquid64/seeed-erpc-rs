#[allow(dead_code)]
use super::{codec, ids, Err};
use no_std_net::Ipv4Addr;
use nom::{bytes::streaming::take, number::streaming};

/// Initializes the layer 3 subsystem.
pub struct AdapterInit {}

impl super::RPC for AdapterInit {
    type ReturnValue = ();
    type Error = ();

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::TCPIP,
            request: ids::TCPIPRequest::AdapterInit.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (_, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::TCPIP
            || hdr.request != ids::TCPIPRequest::AdapterInit.into()
        {
            return Err(Err::NotOurs);
        }

        Ok(())
    }
}

/// Stops any DHCP client management.
pub struct DHCPClientStop {
    pub interface: super::L3Interface,
}

impl super::RPC for DHCPClientStop {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 64>) {
        let interface_id = self.interface as u32;
        buff.extend_from_slice(&interface_id.to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::TCPIP,
            request: ids::TCPIPRequest::DHCPClientStop.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::TCPIP
            || hdr.request != ids::TCPIPRequest::DHCPClientStop.into()
        {
            return Err(Err::NotOurs);
        }

        let (_, ret_val) = streaming::le_i32(data)?;
        Ok(ret_val)
    }
}

/// Starts the DHCP client.
pub struct DHCPClientStart {
    pub interface: super::L3Interface,
}

impl super::RPC for DHCPClientStart {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 64>) {
        let interface_id = self.interface as u32;
        buff.extend_from_slice(&interface_id.to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::TCPIP,
            request: ids::TCPIPRequest::DHCPClientStart.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::TCPIP
            || hdr.request != ids::TCPIPRequest::DHCPClientStart.into()
        {
            return Err(Err::NotOurs);
        }

        let (_, ret_val) = streaming::le_i32(data)?;
        Ok(ret_val)
    }
}

/// Returns the IP configuration the station is using.
pub struct GetIPInfo {
    pub interface: super::L3Interface,
}

impl super::RPC for GetIPInfo {
    type ReturnValue = super::IPInfo;
    type Error = i32;

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::TCPIP,
            request: ids::TCPIPRequest::GetIPInfo.into(),
        }
    }

    fn args(&self, buff: &mut heapless::Vec<u8, 64>) {
        let interface_id = self.interface as u32;
        buff.extend_from_slice(&interface_id.to_le_bytes()).ok();
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::TCPIP
            || hdr.request != ids::TCPIPRequest::GetIPInfo.into()
        {
            return Err(Err::NotOurs);
        }

        let (data, payload_length) = streaming::le_u32(data)?;
        if payload_length != 12 {
            return Err(Err::RPCErr(1));
        }

        let (data, ip) = take(4u8)(data)?;
        let (data, mask) = take(4u8)(data)?;
        let (data, gateway) = take(4u8)(data)?;

        let (_, result) = streaming::le_u32(data)?;
        if result != 0 {
            Err(Err::RPCErr(result as i32))
        } else {
            Ok(super::IPInfo {
                ip: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
                netmask: Ipv4Addr::new(mask[0], mask[1], mask[2], mask[3]),
                gateway: Ipv4Addr::new(gateway[0], gateway[1], gateway[2], gateway[3]),
            })
        }
    }
}
