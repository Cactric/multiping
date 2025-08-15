/// Structs for ICMPv4
// Source: https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol
// TODO: ICMPv6

#[allow(dead_code)]
#[derive(Debug)]
pub enum ICMPMessage {
    ICMPv4(ICMPv4Message),
    ICMPv6(ICMPv6Message),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ICMPv4Message {
    /// Type of control message, including the code
    pub icmpv4_type: ICMPv4Type, // on wire: two bytes (type: u8 and code: u8)
    /// *Big-endian* checksum, calculated from the header (when calculating, this field is 0)
    pub icmpv4_checksum: u16,
    /// Data for the message, including the only-sometimes-used "rest-of-header" header field
    pub icmpv4_data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ICMPv4Type {
    EchoReply { // #0
        identifier: u16,
        sequence_num: u16,
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
        address: u32,
    },
    AlternateHostAddress {}, // #6, deprecated
    // #7 is unassigned and reserved
    EchoRequest { // #8
        identifier: u16,
        sequence_num: u16,
    },
    RouterAdvertisement {}, // #9
    RouterSolicitation {}, // #10
    TimeExceeded { // #11
        code: TimeExceededCode
    },
    BadIPHeader { // #12
        code: BadIPHeaderCode
    },
    Timestamp { // #13
        identifier: u16,
        sequence_num: u16,
        ts_originate: u32,
        ts_receive: u32,
        ts_transmit: u32,
    },
    TimestampReply { // #14
        identifier: u16,
        sequence_num: u16,
        ts_originate: u32,
        ts_receive: u32,
        ts_transmit: u32,
    },
    // TODO: rest of the types above 15, though they're all deprecated, experimental or unassigned
    // (except for Extended Echo Request/Reply)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug)]
pub enum RedirectMsgCode {
    Network, // #0
    Host, // #1
    ToSAndNetwork, // #2
    ToSAndHost, // #3
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum TimeExceededCode {
    ExpiredInTransit, // #0
    FragmentReassemblyTimeExceeded, // #1
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum BadIPHeaderCode {
    PointerIndicatesError, // #0
    MissingRequiredOption, // #1
    BadLength, // #2
}

#[allow(dead_code)]
pub enum IntoICMPv4MessageError {
    UnknownType,
    UnknownCode,
    NotLongEnough,
    OtherError,
}

#[allow(dead_code)]
impl TryFrom<&[u8]> for ICMPv4Message {
    type Error = IntoICMPv4MessageError;

    // TODO: reduce amount of repetition here
    fn try_from(msgbytes: &[u8]) -> Result<Self, Self::Error> {
        match msgbytes[0] { // Match on the type
            0 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::EchoReply {
                    identifier: be_u16(msgbytes, 4),
                    sequence_num: be_u16(msgbytes, 6)
                },
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            3 => {
                let code: DestinationUnreachableCode = parse_unreachable_code(msgbytes[1])?;
                Ok(ICMPv4Message {
                    icmpv4_type: ICMPv4Type::DestinationUnreachable {
                        code: code,
                        length: msgbytes[5],
                        next_hop_mtu: be_u16(msgbytes, 6)
                    }, icmpv4_checksum: be_u16(msgbytes, 2),
                    icmpv4_data: msgbytes[8..].to_vec()
                })
            },
            4 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::SourceQuench {},
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            5 => {
                let code: RedirectMsgCode = parse_redirect_code(msgbytes[1])?;
                Ok(ICMPv4Message {
                    icmpv4_type: ICMPv4Type::RedirectMessage {
                        code: code,
                        address: be_u32(msgbytes, 4)
                    },
                    icmpv4_checksum: be_u16(msgbytes, 2),
                    icmpv4_data: msgbytes[8..].to_vec()
                })
            },
            6 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::SourceQuench {},
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            8 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::EchoRequest {
                    identifier: be_u16(msgbytes, 4),
                    sequence_num: be_u16(msgbytes, 6)
                },
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            9 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::RouterAdvertisement {},
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            10 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::RouterSolicitation {},
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            11 => {
                let code: TimeExceededCode = match msgbytes[1] {
                    0 => TimeExceededCode::ExpiredInTransit,
                    1 => TimeExceededCode::FragmentReassemblyTimeExceeded,
                    _ => return Err(IntoICMPv4MessageError::UnknownCode),
                };
                Ok(ICMPv4Message {
                    icmpv4_type: ICMPv4Type::TimeExceeded {
                        code: code,
                    },
                    icmpv4_checksum: be_u16(msgbytes, 2),
                    icmpv4_data: msgbytes[8..].to_vec()
                })
            },
            12 => {
                let code: BadIPHeaderCode = match msgbytes[1] {
                    0 => BadIPHeaderCode::PointerIndicatesError,
                    1 => BadIPHeaderCode::MissingRequiredOption,
                    2 => BadIPHeaderCode::BadLength,
                    _ => return Err(IntoICMPv4MessageError::UnknownCode),
                };
                Ok(ICMPv4Message {
                    icmpv4_type: ICMPv4Type::BadIPHeader {
                        code: code,
                    },
                    icmpv4_checksum: be_u16(msgbytes, 2),
                    icmpv4_data: msgbytes[8..].to_vec()
                })
            },
            13 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::Timestamp {
                    identifier: be_u16(msgbytes, 4),
                    sequence_num: be_u16(msgbytes, 6),
                    ts_originate: be_u32(msgbytes, 8),
                    ts_receive: be_u32(msgbytes, 12),
                    ts_transmit:  be_u32(msgbytes, 16)
                },
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            14 => Ok(ICMPv4Message {
                icmpv4_type: ICMPv4Type::TimestampReply {
                    identifier: be_u16(msgbytes, 4),
                    sequence_num: be_u16(msgbytes, 6),
                    ts_originate: be_u32(msgbytes, 8),
                    ts_receive: be_u32(msgbytes, 12),
                    ts_transmit:  be_u32(msgbytes, 16)
                },
                icmpv4_checksum: be_u16(msgbytes, 2),
                icmpv4_data: msgbytes[8..].to_vec()
            }),
            _ => Err(IntoICMPv4MessageError::UnknownType)
        }
    }
}

pub fn parse_unreachable_code(value: u8) -> Result<DestinationUnreachableCode, IntoICMPv4MessageError> {
    match value {
        0 => Ok(DestinationUnreachableCode::NetworkUnreachable),
        1 => Ok(DestinationUnreachableCode::HostUnreachable),
        2 => Ok(DestinationUnreachableCode::ProtocolUnreachable),
        3 => Ok(DestinationUnreachableCode::PortUnreachable),
        4 => Ok(DestinationUnreachableCode::FragmentationRequired),
        5 => Ok(DestinationUnreachableCode::SourceRouteFailed),
        6 => Ok(DestinationUnreachableCode::NetworkUnknown),
        7 => Ok(DestinationUnreachableCode::DestHostUnknown),
        8 => Ok(DestinationUnreachableCode::SourceHostIsolated),
        9 => Ok(DestinationUnreachableCode::NetAdministrativelyProhibited),
        10 => Ok(DestinationUnreachableCode::HostAdministrativelyProhibited),
        11 => Ok(DestinationUnreachableCode::NetworkUnreachableForToS),
        12 => Ok(DestinationUnreachableCode::HostUnreachableForToS),
        13 => Ok(DestinationUnreachableCode::CommAdministrativelyProhibited),
        14 => Ok(DestinationUnreachableCode::HostPrecedenceViolation),
        15 => Ok(DestinationUnreachableCode::PrecedenceCuttoffInEffect),
        _ => Err(IntoICMPv4MessageError::UnknownCode)
    }
}

pub fn parse_redirect_code(value: u8) -> Result<RedirectMsgCode, IntoICMPv4MessageError> {
    match value {
        0 => Ok(RedirectMsgCode::Network),
        1 => Ok(RedirectMsgCode::Host),
        2 => Ok(RedirectMsgCode::ToSAndNetwork),
        3 => Ok(RedirectMsgCode::ToSAndHost),
        _ => Err(IntoICMPv4MessageError::UnknownCode)
    }
}


// TODO: write some tests for these (should be easy enough)
/// Construct a big-endian u16 from 2 bytes
fn be_u16(bytes: &[u8], start: usize) -> u16 {
    u16::from_be_bytes(bytes[start..(start+2)].try_into().unwrap())
}
/// Construct a big-endian u32 from four bytes
fn be_u32(bytes: &[u8], start: usize) -> u32 {
    u32::from_be_bytes(bytes[start..(start+4)].try_into().unwrap())
}

/// Construct an echo request message
/// NOTE: identifier and sequence_num here use normal endianness for your platform
#[allow(dead_code)]
pub fn construct_echo_request(identifier: u16, sequence_num: u16, extdata: &[u8]) -> Vec<u8> {
    let msg_type: u8 = 8; // EchoRequest
    let msg_code: u8 = 0;
    // Note that the id and sequence number will be replaced when using a DGRAM socket (rather than RAW), which is currently what the program does
    let be_id = identifier.to_be_bytes();
    let be_seq = sequence_num.to_be_bytes();
    let mut header = [msg_type, msg_code, 0, 0, be_id[0], be_id[1], be_seq[0], be_seq[1]];
    populate_checksum(&mut header);
    let mut message: Vec<u8> = header.to_vec();
    message.append(&mut extdata.to_vec());
    message
}

/// Populates the checksum in the header
#[allow(dead_code)]
pub fn populate_checksum(header: &mut [u8]) {
    let mut total: u32 = 0;
    for b in &mut *header {
        total += *b as u32;
    }
    while !(total < 0xffff) {
        total += total >> 16
    }
    let final_checksum: [u8; 2] = (!total as u16).to_be_bytes();
    header[2] = final_checksum[0];
    header[3] = final_checksum[1];
}

#[derive(Debug)]
pub struct ICMPv6Message {
    // TODO
}
