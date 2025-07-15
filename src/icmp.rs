/// Structs for ICMPv4
// Source: https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol
// TODO: ICMPv6

#[derive(Debug)]
pub enum ICMPMessage {
    ICMPv4(ICMPv4Message),
    ICMPv6(ICMPv6Message),
}

#[derive(Debug)]
pub struct ICMPv4Message {
    /// Type of control message, including the code
    pub icmpv4_type: ICMPv4Type, // on wire: two bytes (type: u8 and code: u8)
    /// *Big-endian* checksum, calculated from the header (when calculating, this field is 0)
    pub icmpv4_checksum: u16,
    /// Data for the message, including the only-sometimes-used "rest-of-header" header field
    pub icmpv4_data: Vec<u8>,
}

#[derive(Debug)]
pub enum ICMPv4Type {
    EchoReply { // #0
        identifier: u8,
        sequence_num: u8,
    },
    // #1 and #2 are unassigned & reserved
    DestinationUnreachable { // #3
        code: DestinationUnreachableCode,
        length: u8,
        next_hop_mtu: u16,
        // The header data is unused
    },
    SourceQuench {}, // #4, deprecated
    RedirectMessage { // #5
        code: RedirectMsgCode,
    },
    AlternateHostAddress {}, // #6, deprecated
    // #7 is unassigned and reserved
    EchoRequest { // #8
        identifier: u8,
        sequence_num: u8,
    },
    RouterAdvertisement {}, // #9
    RouterSolitation {}, // #10
    TimeExceeded { // #11
        code: TimeExceededCode
    },
    BadIPHeader { // #12
        code: BadIPHeaderCode
    },
    Timestamp { // #13
        identifier: u8,
        sequence_num: u8,
        ts_originate: u32,
        ts_receive: u32,
        ts_transmit: u32,
    },
    TimestampReply { // #14
        identifier: u8,
        sequence_num: u8,
        ts_originate: u32,
        ts_receive: u32,
        ts_transmit: u32,
    },
    // TODO: rest of the types above 15, though they're all deprecated, experimental or unassigned
    // (except for Extended Echo Request/Reply)
}

#[derive(Debug)]
pub enum DestinationUnreachableCode {
    NetworkUnreachable, // #0
    HostUnreachable, // #1
    ProtocolUnreachable, // #2
    PortUnreachable, // #3
    FragmentationRequired, // #4. Happens when "Don't Fragment" (DF) flag is set
    SourceRouteFailed, // #5
    NetworkUnknown, // #6
    DestHostUnknown, // #7
    SourceHostIsolated, // #8
    NetAdministrativelyProhibited, //#9
    HostAdministrativelyProhibited, // #10
    NetworkUnreachableForToS, // #11
    HostUnreachableForToS, // #12
    CommAdministrativelyProhibited, // #13
    HostPrecedenceViolation, // #14
    PrecedenceCuttoffInEffect, // #15
}

#[derive(Debug)]
pub enum RedirectMsgCode {
    Network, // #0
    Host, // #1
    ToSAndNetwork, // #2
    ToSAndHost, // #3
}

#[derive(Debug)]
pub enum TimeExceededCode {
    ExpiredInTransit, // #0
    FragmentReassemblyTimeExceeded, // #1
}

#[derive(Debug)]
pub enum BadIPHeaderCode {
    PointerIndicatesError, // #0
    MissingRequiredOption, // #1
    BadLength, // #2
}

pub enum IntoICMPv4MessageError {
    UnknownType,
    UnknownCode,
    NotLongEnough,
    OtherError,
}

impl TryFrom<&[u8]> for ICMPv4Message {
    type Error = IntoICMPv4MessageError;

    fn try_from(msgbytes: &[u8]) -> Result<Self, Self::Error> {
        match msgbytes[0] { // Match on the type
            0 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::EchoReply {
                    identifier: be_u16(&msgbytes[4..=5]),
                    sequence_num: be_u16(&msgbytes[6..=7]) 
                },
                icmpv4_checksum: be_u16(&msgbytes[2..=3]),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            3 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::DestinationUnreachable {
                    code: match msgbytes[1] {
                        0 => DestinationUnreachableCode::NetworkUnreachable,
                        1 => DestinationUnreachableCode::HostUnreachable,
                        2 => DestinationUnreachableCode::ProtocolUnreachable,
                        3 => DestinationUnreachableCode::PortUnreachable,
                        4 => DestinationUnreachableCode::FragmentationRequired,
                        5 => DestinationUnreachableCode::SourceRouteFailed,
                        6 => DestinationUnreachableCode::NetworkUnknown,
                        7 => DestinationUnreachableCode::DestHostUnknown,
                        8 => DestinationUnreachableCode::SourceHostIsolated,
                        9 => DestinationUnreachableCode::NetAdministrativelyProhibited,
                        10 => DestinationUnreachableCode::HostAdministrativelyProhibited,
                        11 => DestinationUnreachableCode::NetworkUnreachableForToS,
                        12 => DestinationUnreachableCode::HostUnreachableForToS,
                        13 => DestinationUnreachableCode::CommAdministrativelyProhibited,
                        14 => DestinationUnreachableCode::HostPrecedenceViolation,
                        15 => DestinationUnreachableCode::PrecedenceCuttoffInEffect,
                    },
                    length: msgbytes[5],
                    next_hop_mtu: be_u16(&msgbytes[6..=7])
                },
                icmpv4_checksum: be_u16(&msgbytes[2..=3]),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            _ => Err(IntoICMPv4MessageError::UnknownType)
        }
    }
}

// TODO: write some tests for these (should be easy enough)
/// Construct a big-endian u16 from a slice of bytes
pub fn be_u16(b: &[u8; 2]) -> u16 {
    (b[0] as u16) << 8 + (b[1] as u16)
}
/// Construct a big-endian u32 from a slice of bytes
pub fn be_u32(b: &[u8; 4]) -> u32 {
    (b[0] as u32) << 24 + (b[1] as u32) << 16 + (b[2] as u32) << 8 + (b[3] as u32)
}

#[derive(Debug)]
pub struct ICMPv6Message {
    // TODO
}
