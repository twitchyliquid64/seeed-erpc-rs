/// eRPC request type
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(unused)]
pub enum MsgType {
    Invocation = 0,
    Oneway = 1,
    Reply = 2,
    Notification = 3,
    Unknown = 255,
}

impl From<u8> for MsgType {
    fn from(mt: u8) -> MsgType {
        match mt {
            0 => MsgType::Invocation,
            1 => MsgType::Oneway,
            2 => MsgType::Reply,
            3 => MsgType::Notification,
            _ => MsgType::Unknown,
        }
    }
}

/// Wio Terminal services
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(unused)]
pub enum Service {
    System = 1,
    BLEHost = 2,
    BLEGap = 3,
    BLEGapBone = 4,
    BLECallback = 13,
    Wifi = 14,
    WifiCallback = 18,
    Unknown = 255,
}

impl From<u8> for Service {
    fn from(mt: u8) -> Service {
        match mt {
            1 => Service::System,
            2 => Service::BLEHost,
            3 => Service::BLEGap,
            4 => Service::BLEGapBone,
            13 => Service::BLECallback,
            14 => Service::Wifi,
            18 => Service::WifiCallback,
            _ => Service::Unknown,
        }
    }
}

/// Wio Terminal request IDs for the System service
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(unused)]
pub enum SystemRequest {
    VersionID = 1,
    AckID = 2,
}

impl From<SystemRequest> for u8 {
    fn from(r: SystemRequest) -> u8 {
        r as u8
    }
}

/// Wio Terminal request IDs for the Wifi service
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(unused)]
pub enum WifiRequest {
    GetMacAddress = 8,
    ScanStart = 64,
    IsScanning = 65,
    ScanGetAP = 66,
    ScanGetNumAPs = 67,
}

impl From<WifiRequest> for u8 {
    fn from(r: WifiRequest) -> u8 {
        r as u8
    }
}
