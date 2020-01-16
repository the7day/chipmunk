// Copyright (c) 2019 E.S.R.Labs. All rights reserved.
//
// NOTICE:  All information contained herein is, and remains
// the property of E.S.R.Labs and its suppliers, if any.
// The intellectual and technical concepts contained herein are
// proprietary to E.S.R.Labs and its suppliers and may be covered
// by German and Foreign Patents, patents in process, and are protected
// by trade secret or copyright law.
// Dissemination of this information or reproduction of this material
// is strictly forbidden unless prior written permission is obtained
// from E.S.R.Labs.
#![allow(clippy::unit_arg)]

use indexer_base::error_reporter::*;
use crate::service_id::*;
use crate::proptest_strategies::*;
use bytes::{ByteOrder, BytesMut, BufMut};
use chrono::{NaiveDateTime};
use chrono::prelude::{Utc, DateTime};
use std::fmt;
use std::fmt::{Formatter};
use std::io;
use std::io::{Error};
use std::rc::Rc;
use serde::Serialize;
use byteorder::{BigEndian, LittleEndian};

use proptest_derive::Arbitrary;
use proptest::prelude::*;

use std::str;

use crate::fibex::{FibexMetadata, FrameId, ApplicationId, ContextId, FrameMetadata};
use crate::dlt_parse::{dlt_fixed_point, dlt_uint, dlt_sint, dlt_fint};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, Serialize, Arbitrary)]
pub enum Endianness {
    /// Little Endian
    Little,
    /// Big Endian
    Big,
}

#[derive(Debug, Clone, PartialEq, Arbitrary, Serialize)]
pub struct DltTimeStamp {
    pub seconds: u32,
    #[proptest(strategy = "0..=1_000_000u32")]
    pub microseconds: u32,
}
impl DltTimeStamp {
    pub fn from_ms(ms: u64) -> Self {
        DltTimeStamp {
            seconds: (ms / 1000) as u32,
            microseconds: (ms % 1000) as u32 * 1000,
        }
    }
}
impl fmt::Display for DltTimeStamp {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let naive: Option<NaiveDateTime> =
            NaiveDateTime::from_timestamp_opt(i64::from(self.seconds), self.microseconds * 1000);
        match naive {
            Some(n) => {
                let datetime: DateTime<Utc> = DateTime::from_utc(n, Utc);
                let system_time: std::time::SystemTime = std::time::SystemTime::from(datetime);
                write!(f, "{}", humantime::format_rfc3339(system_time))
            }
            None => write!(
                f,
                "no valid timestamp for {}s/{}us",
                self.seconds, self.microseconds
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Arbitrary, Serialize)]
pub struct StorageHeader {
    pub timestamp: DltTimeStamp,
    #[proptest(strategy = "\"[a-zA-Z 0-9]{4}\"")]
    pub ecu_id: String,
}
//   EColumn.DATETIME,
//   EColumn.ECUID,
impl fmt::Display for StorageHeader {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}{}{}",
            self.timestamp, DLT_COLUMN_SENTINAL, self.ecu_id
        )
    }
}
trait BytesMutExt {
    fn put_zero_terminated_string(&mut self, s: &str, max: usize);
}
impl BytesMutExt for BytesMut {
    fn put_zero_terminated_string(&mut self, s: &str, max: usize) {
        self.extend_from_slice(s.as_bytes());
        if max > s.len() {
            for _ in 0..(max - s.len()) {
                self.put_u8(0x0);
            }
        }
    }
}
impl StorageHeader {
    #[allow(dead_code)]
    pub fn as_bytes(self: &StorageHeader) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(STORAGE_HEADER_LENGTH);
        buf.extend_from_slice(b"DLT");
        buf.put_u8(0x01);
        buf.put_u32_le(self.timestamp.seconds);
        buf.put_u32_le(self.timestamp.microseconds as u32);
        buf.put_zero_terminated_string(&self.ecu_id[..], 4);
        buf.to_vec()
    }
}
/// The Standard Header shall be in big endian format
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StandardHeader {
    pub version: u8,
    pub endianness: Endianness,
    pub has_extended_header: bool,
    pub message_counter: u8,
    pub ecu_id: Option<String>,
    pub session_id: Option<u32>,
    pub timestamp: Option<u32>,
    pub payload_length: u16,
}

impl StandardHeader {
    pub fn header_type_byte(&self) -> u8 {
        standard_header_type(
            self.has_extended_header,
            self.endianness,
            self.ecu_id.is_some(),
            self.session_id.is_some(),
            self.timestamp.is_some(),
            self.version,
        )
    }
    pub fn overall_length(&self) -> u16 {
        // header length
        let mut length: u16 = HEADER_MIN_LENGTH;
        if self.ecu_id.is_some() {
            length += 4;
        }
        if self.session_id.is_some() {
            length += 4;
        }
        if self.timestamp.is_some() {
            length += 4;
        }
        // add ext header length
        if self.has_extended_header {
            length += EXTENDED_HEADER_LENGTH
        }
        // payload length
        length += self.payload_length;
        length
    }
}

fn standard_header_type(
    has_extended_header: bool,
    endianness: Endianness,
    with_ecu_id: bool,
    with_session_id: bool,
    with_timestamp: bool,
    version: u8,
) -> u8 {
    let mut header_type = 0u8;
    if has_extended_header {
        header_type |= WITH_EXTENDED_HEADER_FLAG
    }
    if endianness == Endianness::Big {
        header_type |= BIG_ENDIAN_FLAG
    }
    if with_ecu_id {
        header_type |= WITH_ECU_ID_FLAG
    }
    if with_session_id {
        header_type |= WITH_SESSION_ID_FLAG
    }
    if with_timestamp {
        header_type |= WITH_TIMESTAMP_FLAG
    }
    header_type |= (version & 0b111) << 5;
    header_type
}
impl StandardHeader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u8,
        endianness: Endianness,
        message_counter: u8,
        has_extended_header: bool,
        payload_length: u16,
        ecu_id: Option<String>,
        session_id: Option<u32>,
        timestamp: Option<u32>,
    ) -> Self {
        StandardHeader {
            // version: header_type_byte >> 5 & 0b111,
            // big_endian: (header_type_byte & BIG_ENDIAN_FLAG) != 0,
            version,
            endianness,
            has_extended_header,
            message_counter,
            ecu_id,
            session_id,
            timestamp,
            payload_length,
        }
    }

    #[allow(dead_code)]
    pub fn header_as_bytes(&self) -> Vec<u8> {
        let header_type_byte = self.header_type_byte();
        let size = calculate_standard_header_length(header_type_byte);
        let mut buf = BytesMut::with_capacity(size as usize);
        buf.put_u8(header_type_byte);
        buf.put_u8(self.message_counter);
        buf.put_u16_be(self.overall_length());
        if let Some(id) = &self.ecu_id {
            buf.put_zero_terminated_string(&id[..], 4);
        }
        if let Some(id) = &self.session_id {
            buf.put_u32_be(*id);
        }
        if let Some(time) = &self.timestamp {
            buf.put_u32_be(*time);
        }
        buf.to_vec()
    }
}

//   EColumn.DATETIME,
//   EColumn.ECUID,
impl fmt::Display for StandardHeader {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}{}", self.version, DLT_COLUMN_SENTINAL,)?;
        if let Some(id) = &self.session_id {
            write!(f, "{}", id)?;
        }
        write!(
            f,
            "{}{}{}",
            DLT_COLUMN_SENTINAL, self.message_counter, DLT_COLUMN_SENTINAL,
        )?;
        if let Some(t) = &self.timestamp {
            write!(f, "{}", t)?;
        }
        write!(f, "{}", DLT_COLUMN_SENTINAL,)?;
        if let Some(id) = &self.ecu_id {
            write!(f, "{}", id)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Serialize, Arbitrary)]
pub enum LogLevel {
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Verbose,
    #[proptest(strategy = "(7..=15u8).prop_map(LogLevel::Invalid)")]
    Invalid(u8),
}
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            LogLevel::Fatal => f.write_str("FATAL"),
            LogLevel::Error => f.write_str("Error"),
            LogLevel::Warn => f.write_str("WARN"),
            LogLevel::Info => f.write_str("INFO"),
            LogLevel::Debug => f.write_str("DEBUG"),
            LogLevel::Verbose => f.write_str("VERBOSE"),
            LogLevel::Invalid(v) => write!(f, "INVALID (0x{:02X?})", v),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Arbitrary, Serialize)]
pub enum ApplicationTraceType {
    Variable,
    FunctionIn,
    FunctionOut,
    State,
    Vfb,
    #[proptest(strategy = "(6..15u8).prop_map(ApplicationTraceType::Invalid)")]
    Invalid(u8),
}
impl fmt::Display for ApplicationTraceType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            ApplicationTraceType::Variable => f.write_str("VARIABLE"),
            ApplicationTraceType::FunctionIn => f.write_str("FUNC_IN"),
            ApplicationTraceType::FunctionOut => f.write_str("FUNC_OUT"),
            ApplicationTraceType::State => f.write_str("STATE"),
            ApplicationTraceType::Vfb => f.write_str("VFB"),
            ApplicationTraceType::Invalid(n) => write!(f, "invalid({})", n),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Arbitrary, Serialize)]
pub enum NetworkTraceType {
    Ipc,
    Can,
    Flexray,
    Most,
    Ethernet,
    Someip,
    Invalid,
    #[proptest(strategy = "(7..15u8).prop_map(NetworkTraceType::UserDefined)")]
    UserDefined(u8),
}
impl fmt::Display for NetworkTraceType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            NetworkTraceType::Invalid => f.write_str("INVALID"),
            NetworkTraceType::Ipc => f.write_str("IPC"),
            NetworkTraceType::Can => f.write_str("CAN"),
            NetworkTraceType::Flexray => f.write_str("FLEXRAY"),
            NetworkTraceType::Most => f.write_str("MOST"),
            NetworkTraceType::Ethernet => f.write_str("ETHERNET"),
            NetworkTraceType::Someip => f.write_str("SOMEIP"),
            NetworkTraceType::UserDefined(v) => write!(f, "USERDEFINED({})", v),
        }
    }
}
const CTRL_TYPE_REQUEST: u8 = 0x1;
const CTRL_TYPE_RESPONSE: u8 = 0x2;
#[derive(Debug, PartialEq, Clone, Arbitrary, Serialize)]
pub enum ControlType {
    Request,  // represented by 0x1
    Response, // represented by 0x2
    #[proptest(strategy = "(3..15u8).prop_map(ControlType::Unknown)")]
    Unknown(u8),
}
impl ControlType {
    fn value(&self) -> u8 {
        match *self {
            ControlType::Request => CTRL_TYPE_REQUEST,
            ControlType::Response => CTRL_TYPE_RESPONSE,
            ControlType::Unknown(n) => n,
        }
    }
    pub fn from_value(t: u8) -> Self {
        match t {
            CTRL_TYPE_REQUEST => ControlType::Request,
            CTRL_TYPE_RESPONSE => ControlType::Response,
            t => ControlType::Unknown(t),
        }
    }
}
impl fmt::Display for ControlType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            ControlType::Request => f.write_str("REQ"),
            ControlType::Response => f.write_str("RES"),
            ControlType::Unknown(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Arbitrary, Serialize)]
pub enum MessageType {
    Log(LogLevel),
    ApplicationTrace(ApplicationTraceType),
    NetworkTrace(NetworkTraceType),
    Control(ControlType),
    #[proptest(strategy = "((0b100u8..0b111u8),(0..0b1111u8)).prop_map(MessageType::Unknown)")]
    Unknown((u8, u8)),
}

impl MessageType {
    fn try_new_from_fibex_message_info(message_info: &str) -> Option<MessageType> {
        Some(MessageType::Log(match message_info {
            "DLT_LOG_FATAL" => LogLevel::Fatal,
            "DLT_LOG_ERROR" => LogLevel::Error,
            "DLT_LOG_WARN" => LogLevel::Warn,
            "DLT_LOG_INFO" => LogLevel::Info,
            "DLT_LOG_DEBUG" => LogLevel::Debug,
            "DLT_LOG_VERBOSE" => LogLevel::Verbose,
            _ => return None,
        }))
    }
}

pub const DLT_TYPE_LOG: u8 = 0b000;
pub const DLT_TYPE_APP_TRACE: u8 = 0b001;
pub const DLT_TYPE_NW_TRACE: u8 = 0b010;
pub const DLT_TYPE_CONTROL: u8 = 0b011;

/// The Extended Header shall be in big endian format
#[derive(Debug, Clone, PartialEq, Arbitrary, Serialize)]
pub struct ExtendedHeader {
    pub verbose: bool,
    #[proptest(strategy = "0..=5u8")]
    pub argument_count: u8,
    pub message_type: MessageType,

    #[proptest(strategy = "\"[a-zA-Z]{1,3}\"")]
    pub application_id: String,
    #[proptest(strategy = "\"[a-zA-Z]{1,3}\"")]
    pub context_id: String,
}

impl ExtendedHeader {
    #[allow(dead_code)]
    pub fn as_bytes(self: &ExtendedHeader) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(EXTENDED_HEADER_LENGTH as usize);
        buf.put_u8(u8::from(&self.message_type) | if self.verbose { 1 } else { 0 });
        buf.put_u8(self.argument_count);
        buf.put_zero_terminated_string(&self.application_id[..], 4);
        buf.put_zero_terminated_string(&self.context_id[..], 4);
        buf.to_vec()
    }
    pub fn skip_with_level(self: &ExtendedHeader, level: LogLevel) -> bool {
        match self.message_type {
            MessageType::Log(n) => match (n, level) {
                (LogLevel::Invalid(a), LogLevel::Invalid(b)) => a < b,
                (LogLevel::Invalid(_), _) => false,
                (_, LogLevel::Invalid(_)) => true,
                _ => level < n,
            },
            _ => false,
        }
    }
}

/// Fixed-Point representation. only supports 32 bit and 64 bit values
/// according to the spec 128 bit are possible but we don't support it
#[derive(Debug, PartialEq, Clone, Arbitrary, Serialize)]
pub enum FixedPointValue {
    I32(i32),
    I64(i64),
}
pub fn fixed_point_value_width(v: &FixedPointValue) -> usize {
    match v {
        FixedPointValue::I32(_) => 4,
        FixedPointValue::I64(_) => 8,
    }
}
#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum Value {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
    StringVal(String),
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Arbitrary, Serialize)]
pub enum StringCoding {
    ASCII,
    UTF8,
}
#[derive(Debug, Clone, PartialEq, Copy, Arbitrary, Serialize)]
pub enum FloatWidth {
    Width32 = 32,
    Width64 = 64,
}
pub fn float_width_to_type_length(width: FloatWidth) -> TypeLength {
    match width {
        FloatWidth::Width32 => TypeLength::BitLength32,
        FloatWidth::Width64 => TypeLength::BitLength64,
    }
}

#[derive(Debug, Clone, PartialEq, Copy, Arbitrary, Serialize)]
pub enum TypeLength {
    BitLength8 = 8,
    BitLength16 = 16,
    BitLength32 = 32,
    BitLength64 = 64,
    BitLength128 = 128,
}

#[derive(Debug, Clone, PartialEq, Arbitrary, Serialize)]
pub enum TypeInfoKind {
    Bool,
    #[proptest(strategy = "signed_strategy()")]
    Signed(TypeLength),
    SignedFixedPoint(FloatWidth),
    #[proptest(strategy = "unsigned_strategy()")]
    Unsigned(TypeLength),
    UnsignedFixedPoint(FloatWidth),
    Float(FloatWidth),
    // Array, NYI
    StringType,
    Raw,
}

///
/// TypeInfo is a 32 bit field. It is encoded the following way:
///     * Bit0-3    Type Length (TYLE)  -> TypeKindInfo
///     * Bit 4     Type Bool (BOOL)    -> TypeKindInfo
///     * Bit 5     Type Signed (SINT)  -> TypeKindInfo
///     * Bit 6     Type Unsigned (UINT) -> TypeKindInfo
///     * Bit 7     Type Float (FLOA)   -> TypeKindInfo
///     * Bit 8     Type Array (ARAY)   -> TypeKindInfo
///     * Bit 9     Type String (STRG)  -> TypeKindInfo
///     * Bit 10    Type Raw (RAWD)     -> TypeKindInfo
///     * Bit 11    Variable Info (VARI)
///     * Bit 12    Fixed Point (FIXP)  -> TypeKindInfo
///     * Bit 13    Trace Info (TRAI)
///     * Bit 14    Type Struct (STRU)  -> TypeKindInfo
///     * Bit15–17  String Coding (SCOD)
///     * Bit18–31  reserved for future use
///
/// has_variable_info: If Variable Info (VARI) is set, the name and the unit of a variable can be added.
/// Both always contain a length information field and a field with the text (of name or unit).
/// The length field contains the number of characters of the associated name or unit filed.
/// The unit information is to add only in some data types.
#[derive(Debug, Clone, PartialEq, Arbitrary, Serialize)]
pub struct TypeInfo {
    pub kind: TypeInfoKind,
    pub coding: StringCoding,
    pub has_variable_info: bool,
    pub has_trace_info: bool,
}
impl TypeInfo {
    pub fn type_length_bits_float(len: FloatWidth) -> u32 {
        match len {
            FloatWidth::Width32 => 0b011,
            FloatWidth::Width64 => 0b100,
        }
    }
    pub fn type_length_bits(len: TypeLength) -> u32 {
        match len {
            TypeLength::BitLength8 => 0b001,
            TypeLength::BitLength16 => 0b010,
            TypeLength::BitLength32 => 0b011,
            TypeLength::BitLength64 => 0b100,
            TypeLength::BitLength128 => 0b101,
        }
    }
    pub fn type_width(self: &TypeInfo) -> usize {
        match self.kind {
            TypeInfoKind::Signed(v) => v as usize,
            TypeInfoKind::SignedFixedPoint(v) => v as usize,
            TypeInfoKind::Unsigned(v) => v as usize,
            TypeInfoKind::UnsignedFixedPoint(v) => v as usize,
            TypeInfoKind::Float(v) => v as usize,
            _ => 0,
        }
    }
    pub fn is_fixed_point(self: &TypeInfo) -> bool {
        match self.kind {
            TypeInfoKind::SignedFixedPoint(_) => true,
            TypeInfoKind::UnsignedFixedPoint(_) => true,
            _ => false,
        }
    }
    pub fn as_bytes<T: ByteOrder>(self: &TypeInfo) -> Vec<u8> {
        // println!("TypeInfo::as_bytes: {:?}", self);
        let mut info: u32 = 0;
        // encode length
        match self.kind {
            TypeInfoKind::Float(len) => info |= TypeInfo::type_length_bits_float(len),
            TypeInfoKind::Signed(len) => info |= TypeInfo::type_length_bits(len),
            TypeInfoKind::SignedFixedPoint(len) => info |= TypeInfo::type_length_bits_float(len),
            TypeInfoKind::Unsigned(len) => info |= TypeInfo::type_length_bits(len),
            TypeInfoKind::UnsignedFixedPoint(len) => info |= TypeInfo::type_length_bits_float(len),
            TypeInfoKind::Bool => info |= TypeInfo::type_length_bits(TypeLength::BitLength8),
            _ => (),
        }
        match self.kind {
            TypeInfoKind::Bool => info |= TYPE_INFO_BOOL_FLAG,
            TypeInfoKind::Signed(_) => info |= TYPE_INFO_SINT_FLAG,
            TypeInfoKind::SignedFixedPoint(_) => info |= TYPE_INFO_SINT_FLAG,
            TypeInfoKind::Unsigned(_) => info |= TYPE_INFO_UINT_FLAG,
            TypeInfoKind::UnsignedFixedPoint(_) => info |= TYPE_INFO_UINT_FLAG,
            TypeInfoKind::Float(_) => info |= TYPE_INFO_FLOAT_FLAG,
            // TypeInfoKind::Array => info |= TYPE_INFO_ARRAY_FLAG,
            TypeInfoKind::StringType => info |= TYPE_INFO_STRING_FLAG,
            TypeInfoKind::Raw => info |= TYPE_INFO_RAW_FLAG,
        }
        if self.has_variable_info {
            info |= TYPE_INFO_VARIABLE_INFO
        }
        if self.is_fixed_point() {
            info |= TYPE_INFO_FIXED_POINT_FLAG
        }
        if self.has_trace_info {
            info |= TYPE_INFO_TRACE_INFO_FLAG
        }
        match self.coding {
            StringCoding::ASCII => info |= 0b000 << 15,
            StringCoding::UTF8 => info |= 0b001 << 15,
        }

        let mut buf = BytesMut::with_capacity(4);
        let mut b = [0; 4];
        T::write_u32(&mut b, info);
        buf.put_slice(&b);
        buf.to_vec()
    }
}
///    Bit Representation               0b0011_1111_1111_1111_1111
///    string coding .......................^^_^||| |||| |||| ||||
///    type struct .............................^|| |||| |||| ||||
///    trace info ...............................^| |||| |||| ||||
///    fixed point ...............................^ |||| |||| ||||
///    variable info................................^||| |||| ||||
///    type raw .....................................^|| |||| ||||
///    type string ...................................^| |||| ||||
///    type array .....................................^ |||| ||||
///    type float .......................................^||| ||||
///    type unsigned .....................................^|| ||||
///    type signed ........................................^| ||||
///    type bool ...........................................^ ||||
///    type length ...........................................^^^^
impl TryFrom<u32> for TypeInfo {
    type Error = Error;
    fn try_from(info: u32) -> Result<TypeInfo, Error> {
        fn type_len(info: u32) -> Result<TypeLength, Error> {
            match info & 0b1111 {
                0x01 => Ok(TypeLength::BitLength8),
                0x02 => Ok(TypeLength::BitLength16),
                0x03 => Ok(TypeLength::BitLength32),
                0x04 => Ok(TypeLength::BitLength64),
                0x05 => Ok(TypeLength::BitLength128),
                v => Err(Error::new(
                    io::ErrorKind::Other,
                    format!("Unknown type_len in TypeInfo {:b}", v),
                )),
            }
        }
        fn type_len_float(info: u32) -> Result<FloatWidth, Error> {
            match info & 0b1111 {
                0x03 => Ok(FloatWidth::Width32),
                0x04 => Ok(FloatWidth::Width64),
                v => Err(Error::new(
                    io::ErrorKind::Other,
                    format!("Unknown type_len_float in TypeInfo {:b}", v),
                )),
            }
        }

        let is_fixed_point = (info & TYPE_INFO_FIXED_POINT_FLAG) != 0;
        let kind = match (info >> 4) & 0b111_1111 {
            0b000_0001 => Ok(TypeInfoKind::Bool),
            0b000_0010 => Ok(if is_fixed_point {
                TypeInfoKind::SignedFixedPoint(type_len_float(info)?)
            } else {
                TypeInfoKind::Signed(type_len(info)?)
            }),
            0b000_0100 => Ok(if is_fixed_point {
                TypeInfoKind::UnsignedFixedPoint(type_len_float(info)?)
            } else {
                TypeInfoKind::Unsigned(type_len(info)?)
            }),
            0b000_1000 => Ok(TypeInfoKind::Float(type_len_float(info)?)),
            // 0b001_0000 => Ok(TypeInfoKind::Array),
            0b010_0000 => Ok(TypeInfoKind::StringType),
            0b100_0000 => Ok(TypeInfoKind::Raw),
            v => Err(Error::new(
                io::ErrorKind::Other,
                format!("Unknown TypeInfoKind in TypeInfo {:b}", v),
            )),
        }?;
        let coding = match (info >> 15) & 0b111 {
            0x00 => (StringCoding::ASCII),
            0x01 => (StringCoding::UTF8),
            _ => {
                // Unknown coding in TypeInfo, assume UTF8
                (StringCoding::UTF8)
            }
        };
        Ok(TypeInfo {
            has_variable_info: (info & TYPE_INFO_VARIABLE_INFO) != 0,
            has_trace_info: (info & TYPE_INFO_TRACE_INFO_FLAG) != 0,
            kind,
            coding,
        })
    }
}
/// The following equation defines the relation between the logical value (log_v) and
/// the physical value (phy_v), offset and quantization:
///     log_v = phy_v * quantization + offset
///
/// * phy_v is what we received in the dlt message
/// * log_v is the real value
/// example: the degree celcius is transmitted,
/// quantization = 0.01, offset = -50
/// now the transmitted value phy_v = (log_v - offset)/quantization = 7785
///
/// The width depends on the TYLE value
///     * i32 bit if Type Length (TYLE) equals 1,2 or 3
///     * i64 bit if Type Length (TYLE) equals 4
///     * i128 bit if Type Length (TYLE) equals 5 (unsupported)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixedPoint {
    pub quantization: f32,
    pub offset: FixedPointValue,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Argument {
    pub type_info: TypeInfo,
    pub name: Option<String>,
    pub unit: Option<String>,
    pub fixed_point: Option<FixedPoint>,
    pub value: Value,
}
impl Argument {
    fn value_as_f64(&self) -> Option<f64> {
        match self.value {
            Value::I8(v) => Some(v as f64),
            Value::I16(v) => Some(v as f64),
            Value::I32(v) => Some(v as f64),
            Value::I64(v) => Some(v as f64),
            Value::U8(v) => Some(v as f64),
            Value::U16(v) => Some(v as f64),
            Value::U32(v) => Some(v as f64),
            Value::U64(v) => Some(v as f64),
            _ => None,
        }
    }
    fn log_v(&self) -> Option<u64> {
        match &self.fixed_point {
            Some(FixedPoint {
                quantization,
                offset,
            }) => {
                if let Some(value) = self.value_as_f64() {
                    match offset {
                        FixedPointValue::I32(v) => {
                            Some((value * *quantization as f64) as u64 + *v as u64)
                        }
                        FixedPointValue::I64(v) => {
                            Some((value * *quantization as f64) as u64 + *v as u64)
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub(crate) fn to_real_value(&self) -> Option<u64> {
        match (&self.type_info.kind, &self.fixed_point) {
            (TypeInfoKind::SignedFixedPoint(_), Some(_)) => self.log_v(),
            (TypeInfoKind::UnsignedFixedPoint(_), Some(_)) => self.log_v(),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn valid(&self) -> bool {
        let mut valid = true;
        match self.type_info.kind {
            TypeInfoKind::Bool => match self.value {
                Value::Bool(_) => (),
                _ => valid = false,
            },
            TypeInfoKind::Float(FloatWidth::Width32) => match self.value {
                Value::F32(_) => (),
                _ => valid = false,
            },
            TypeInfoKind::Float(FloatWidth::Width64) => match self.value {
                Value::F64(_) => (),
                _ => valid = false,
            },
            _ => (),
        }
        valid
    }
    pub fn len<T: ByteOrder>(&self) -> usize {
        self.as_bytes::<T>().len()
    }
    pub fn is_empty<T: ByteOrder>(&self) -> bool {
        self.len::<T>() == 0
    }
    fn mut_buf_with_typeinfo_name<T: ByteOrder>(
        &self,
        info: &TypeInfo,
        name: &Option<String>,
    ) -> BytesMut {
        let mut capacity = TYPE_INFO_LENGTH + info.type_width();
        if let Some(n) = name {
            capacity += 2 /* length name */ + n.len() + 1;
        }
        let mut buf = BytesMut::with_capacity(capacity);
        buf.extend_from_slice(&info.as_bytes::<T>()[..]);
        if let Some(n) = name {
            #[allow(deprecated)]
            buf.put_u16::<T>(n.len() as u16 + 1);
            buf.extend_from_slice(n.as_bytes());
            buf.put_u8(0x0); // null termination
        }
        buf
    }
    fn mut_buf_with_typeinfo_name_unit<T: ByteOrder>(
        &self,
        info: &TypeInfo,
        name: &Option<String>,
        unit: &Option<String>,
        fixed_point: &Option<FixedPoint>,
    ) -> BytesMut {
        // println!("mut_buf_with_typeinfo_name_unit {:?}/{:?}", name, unit);
        let mut capacity = TYPE_INFO_LENGTH;
        if info.has_variable_info {
            if let Some(n) = name {
                capacity += 2 /* length name */ + n.len() + 1;
            } else {
                capacity += 2 + 1; // only length field and \0 termination
            }
            if let Some(u) = unit {
                capacity += 2 /* length unit */ + u.len() + 1;
            } else {
                capacity += 2 + 1; // only length field and \0 termination
            }
        }
        if let Some(fp) = fixed_point {
            capacity += 4 /* quantization */ + fixed_point_value_width(&fp.offset);
        }
        capacity += info.type_width();
        let mut buf = BytesMut::with_capacity(capacity);
        buf.extend_from_slice(&info.as_bytes::<T>()[..]);
        if info.has_variable_info {
            if let Some(n) = name {
                #[allow(deprecated)]
                buf.put_u16::<T>(n.len() as u16 + 1);
            // println!("put name len: {:02X?}", buf.to_vec());
            } else {
                #[allow(deprecated)]
                buf.put_u16::<T>(1u16);
            }
            if let Some(u) = unit {
                #[allow(deprecated)]
                buf.put_u16::<T>(u.len() as u16 + 1);
            // println!("put unit len: {:02X?}", buf.to_vec());
            } else {
                #[allow(deprecated)]
                buf.put_u16::<T>(1u16);
            }
            if let Some(n) = name {
                buf.extend_from_slice(n.as_bytes());
                buf.put_u8(0x0); // null termination
                                 // println!("put name: {:02X?}", buf.to_vec());
            } else {
                buf.put_u8(0x0); // only null termination
            }
            if let Some(u) = unit {
                buf.extend_from_slice(u.as_bytes());
                buf.put_u8(0x0); // null termination
                                 // println!("put unit: {:02X?}", buf.to_vec());
            } else {
                buf.put_u8(0x0); // only null termination
            }
        }
        if let Some(fp) = fixed_point {
            #[allow(deprecated)]
            buf.put_f32::<T>(fp.quantization);
            match fp.offset {
                FixedPointValue::I32(v) => {
                    #[allow(deprecated)]
                    buf.put_i32::<T>(v);
                }
                FixedPointValue::I64(v) => {
                    #[allow(deprecated)]
                    buf.put_i64::<T>(v);
                }
            }
        }
        // println!("typeinfo + name + unit as bytes: {:02X?}", buf.to_vec());
        buf
    }
    #[allow(dead_code)]
    pub fn as_bytes<T: ByteOrder>(self: &Argument) -> Vec<u8> {
        match self.type_info.kind {
            TypeInfoKind::Bool => {
                let mut buf = self.mut_buf_with_typeinfo_name::<T>(&self.type_info, &self.name);
                let v = if self.value == Value::Bool(true) {
                    0x1
                } else {
                    0x0
                };
                buf.put_u8(v);
                dbg_bytes(buf.len(), "bool argument", &buf.to_vec()[..]);
                buf.to_vec()
            }
            TypeInfoKind::Signed(_) => {
                let mut buf = self.mut_buf_with_typeinfo_name_unit::<T>(
                    &self.type_info,
                    &self.name,
                    &self.unit,
                    &self.fixed_point,
                );
                put_signed_value::<T>(&self.value, &mut buf);
                dbg_bytes(buf.len(), "signed argument", &buf.to_vec()[..]);
                buf.to_vec()
            }
            TypeInfoKind::SignedFixedPoint(_) => {
                let mut buf = self.mut_buf_with_typeinfo_name_unit::<T>(
                    &self.type_info,
                    &self.name,
                    &self.unit,
                    &self.fixed_point,
                );
                put_signed_value::<T>(&self.value, &mut buf);
                dbg_bytes(buf.len(), "signed fixed point argument", &buf.to_vec()[..]);
                buf.to_vec()
            }
            TypeInfoKind::Unsigned(_) => {
                let mut buf = self.mut_buf_with_typeinfo_name_unit::<T>(
                    &self.type_info,
                    &self.name,
                    &self.unit,
                    &self.fixed_point,
                );
                put_unsigned_value::<T>(&self.value, &mut buf);
                dbg_bytes(buf.len(), "unsigned argument", &buf.to_vec()[..]);
                buf.to_vec()
            }
            TypeInfoKind::UnsignedFixedPoint(_) => {
                let mut buf = self.mut_buf_with_typeinfo_name_unit::<T>(
                    &self.type_info,
                    &self.name,
                    &self.unit,
                    &self.fixed_point,
                );
                put_unsigned_value::<T>(&self.value, &mut buf);
                dbg_bytes(buf.len(), "unsigned FP argument", &buf.to_vec()[..]);
                buf.to_vec()
            }
            TypeInfoKind::Float(_) => {
                fn write_value<T: ByteOrder>(value: &Value, buf: &mut BytesMut) {
                    match value {
                        Value::F32(v) => {
                            let mut b = [0; 4];
                            T::write_f32(&mut b, *v);
                            buf.put_slice(&b)
                        }
                        Value::F64(v) => {
                            let mut b = [0; 8];
                            T::write_f64(&mut b, *v);
                            buf.put_slice(&b)
                        }
                        _ => (),
                    }
                }
                let mut buf = self.mut_buf_with_typeinfo_name_unit::<T>(
                    &self.type_info,
                    &self.name,
                    &self.unit,
                    &self.fixed_point,
                );
                write_value::<T>(&self.value, &mut buf);
                dbg_bytes(buf.len(), "float argument", &buf.to_vec()[..]);
                buf.to_vec()
            }
            // TypeInfoKind::Array => {
            //     // TODO dlt array type not yet implemented NYI
            //     eprintln!("found dlt array type...not yet supported");
            //     BytesMut::with_capacity(STORAGE_HEADER_LENGTH).to_vec()
            // }
            TypeInfoKind::StringType => {
                match (self.type_info.has_variable_info, &self.name) {
                    (true, Some(var_name)) => {
                        match &self.value {
                            Value::StringVal(s) => {
                                let name_len_with_termination: u16 = var_name.len() as u16 + 1;
                                let mut buf = BytesMut::with_capacity(
                                    TYPE_INFO_LENGTH +
                                    2 /* length string */ +
                                    2 /* length name */ +
                                    name_len_with_termination as usize +
                                    s.len() + 1,
                                );
                                buf.extend_from_slice(&self.type_info.as_bytes::<T>()[..]);
                                #[allow(deprecated)]
                                buf.put_u16::<T>(s.len() as u16 + 1);
                                #[allow(deprecated)]
                                buf.put_u16::<T>(name_len_with_termination);
                                buf.extend_from_slice(var_name.as_bytes());
                                buf.put_u8(0x0); // null termination
                                buf.extend_from_slice(s.as_bytes());
                                buf.put_u8(0x0); // null termination
                                dbg_bytes(
                                    buf.len(),
                                    "StringType with variable info",
                                    &buf.to_vec()[..],
                                );
                                buf.to_vec()
                            }
                            v => {
                                error!("found invalid dlt entry for StringType ({:?}", v);
                                BytesMut::with_capacity(0).to_vec()
                            }
                        }
                    }
                    (false, None) => {
                        match &self.value {
                            Value::StringVal(s) => {
                                let mut buf = BytesMut::with_capacity(
                                    TYPE_INFO_LENGTH +
                                    2 /* length string */ +
                                    s.len() + 1,
                                );
                                buf.extend_from_slice(&self.type_info.as_bytes::<T>()[..]);
                                #[allow(deprecated)]
                                buf.put_u16::<T>(s.len() as u16 + 1);
                                buf.extend_from_slice(s.as_bytes());
                                buf.put_u8(0x0); // null termination
                                dbg_bytes(
                                    buf.len(),
                                    "StringType, no variable info",
                                    &buf.to_vec()[..],
                                );
                                buf.to_vec()
                            }
                            _ => {
                                error!("found invalid dlt entry for StringType ({:?}", self);
                                BytesMut::with_capacity(0).to_vec()
                            }
                        }
                    }
                    _ => {
                        error!("found invalid dlt entry ({:?}", self);
                        BytesMut::with_capacity(0).to_vec()
                    }
                }
            }
            TypeInfoKind::Raw => {
                match (self.type_info.has_variable_info, &self.name) {
                    (true, Some(var_name)) => {
                        match &self.value {
                            Value::Raw(bytes) => {
                                let name_len_with_termination: u16 = var_name.len() as u16 + 1;
                                let mut buf = BytesMut::with_capacity(
                                    TYPE_INFO_LENGTH +
                                    2 /* length bytes */ +
                                    2 /* length name */ +
                                    name_len_with_termination as usize +
                                    bytes.len(),
                                );
                                buf.extend_from_slice(&self.type_info.as_bytes::<T>()[..]);
                                #[allow(deprecated)]
                                buf.put_u16::<T>(bytes.len() as u16);
                                #[allow(deprecated)]
                                buf.put_u16::<T>(name_len_with_termination);
                                buf.extend_from_slice(var_name.as_bytes());
                                buf.put_u8(0x0); // null termination
                                buf.extend_from_slice(bytes);
                                dbg_bytes(buf.len(), "Raw, with variable info", &buf.to_vec()[..]);
                                buf.to_vec()
                            }
                            _ => {
                                error!("found invalid dlt entry for Raw ({:?}", self);
                                BytesMut::with_capacity(0).to_vec()
                            }
                        }
                    }
                    (false, None) => {
                        match &self.value {
                            Value::Raw(bytes) => {
                                let mut buf = BytesMut::with_capacity(
                                    TYPE_INFO_LENGTH +
                                    2 /* length string */ +
                                    bytes.len(),
                                );
                                buf.extend_from_slice(&self.type_info.as_bytes::<T>()[..]);
                                #[allow(deprecated)]
                                buf.put_u16::<T>(bytes.len() as u16);
                                buf.extend_from_slice(bytes);
                                dbg_bytes(buf.len(), "Raw, no variable info", &buf.to_vec()[..]);
                                buf.to_vec()
                            }
                            _ => {
                                error!("found invalid dlt entry for Raw ({:?}", self);
                                BytesMut::with_capacity(0).to_vec()
                            }
                        }
                    }
                    _ => {
                        error!("found invalid dlt entry ({:?}", self);
                        BytesMut::with_capacity(0).to_vec()
                    }
                }
            }
        }
    }
}
fn put_unsigned_value<T: ByteOrder>(value: &Value, buf: &mut BytesMut) {
    match value {
        Value::U8(v) => buf.put_u8(*v),
        Value::U16(v) => {
            let mut b = [0; 2];
            T::write_u16(&mut b, *v);
            buf.put_slice(&b)
        }
        Value::U32(v) => {
            let mut b = [0; 4];
            T::write_u32(&mut b, *v);
            buf.put_slice(&b)
        }
        Value::U64(v) => {
            let mut b = [0; 8];
            T::write_u64(&mut b, *v);
            buf.put_slice(&b)
        }
        Value::U128(v) => {
            let mut b = [0; 16];
            T::write_u128(&mut b, *v);
            buf.put_slice(&b);
        }
        _ => (),
    }
}
fn put_signed_value<T: ByteOrder>(value: &Value, buf: &mut BytesMut) {
    match value {
        Value::I8(v) => buf.put_i8(*v),
        Value::I16(v) => {
            let mut b = [0; 2];
            T::write_i16(&mut b, *v);
            buf.put_slice(&b)
        }
        Value::I32(v) => {
            let mut b = [0; 4];
            T::write_i32(&mut b, *v);
            buf.put_slice(&b)
        }
        Value::I64(v) => {
            let mut b = [0; 8];
            T::write_i64(&mut b, *v);
            buf.put_slice(&b)
        }
        Value::I128(v) => {
            let mut b = [0; 16];
            T::write_i128(&mut b, *v);
            buf.put_slice(&b);
        }
        v => warn!("not a valid signed value: {:?}", v),
    }
}

/// There are 3 different types of payload:
///     * one for verbose messages,
///     * one for non-verbose messages,
///     * one for control-messages
///
/// For Non-Verbose mode (without Extended Header), a fibex file provides an
/// additional description for the payload.
/// With the combination of a Message ID and an external fibex description,
/// following information is be recoverable (otherwise provided
/// in the Extended Header):
///     * Message Type (MSTP)
///     * Message Info (MSIN)
///     * Number of arguments (NOAR)
///     * Application ID (APID)
///     * Context ID (CTID)
///
/// Control messages are normal Dlt messages with a Standard Header, an Extended Header,
/// and payload. The payload contains of the Service ID and the contained parameters.
///
#[derive(Debug, Clone, PartialEq, Arbitrary, Serialize)]
pub enum PayloadContent {
    #[proptest(strategy = "argument_vector_strategy().prop_map(PayloadContent::Verbose)")]
    Verbose(Vec<Argument>),
    #[proptest(
        strategy = "(0..10u32, prop::collection::vec(any::<u8>(), 0..5)).prop_map(|(a, b)| PayloadContent::NonVerbose(a,b))"
    )]
    NonVerbose(u32, Vec<u8>), // (message_id, payload)
    #[proptest(
        strategy = "(any::<ControlType>(), prop::collection::vec(any::<u8>(), 0..5)).prop_map(|(a, b)| PayloadContent::ControlMsg(a,b))"
    )]
    ControlMsg(ControlType, Vec<u8>),
}
fn payload_content_len<T: ByteOrder>(content: &PayloadContent) -> usize {
    match content {
        PayloadContent::Verbose(args) => args.iter().fold(0usize, |mut sum, arg| {
            sum += arg.len::<T>();
            sum
        }),
        PayloadContent::NonVerbose(_id, payload) => 4usize + payload.len(),
        PayloadContent::ControlMsg(_id, payload) => 1usize + payload.len(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Payload2 {
    pub payload_content: PayloadContent,
}
impl Payload2 {
    pub fn arg_count(&self) -> u8 {
        match &self.payload_content {
            PayloadContent::Verbose(args) => std::cmp::min(args.len() as u8, u8::max_value()),
            _ => 0,
        }
    }
    #[allow(dead_code)]
    pub(crate) fn is_verbose(&self) -> bool {
        match self.payload_content {
            PayloadContent::Verbose(_) => true,
            _ => false,
        }
    }
    #[allow(dead_code)]
    fn is_non_verbose(&self) -> bool {
        match self.payload_content {
            PayloadContent::NonVerbose(_, _) => true,
            _ => false,
        }
    }
    #[allow(dead_code)]
    fn is_control_request(&self) -> Option<bool> {
        match self.payload_content {
            PayloadContent::ControlMsg(ControlType::Request, _) => Some(true),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_bytes<T: ByteOrder>(&self) -> Vec<u8> {
        // println!("...Payload2::as_bytes for {:?}", self);
        let mut buf = BytesMut::with_capacity(payload_content_len::<T>(&self.payload_content));
        match &self.payload_content {
            PayloadContent::Verbose(args) => {
                // println!(
                //     "...Payload2::as_bytes, writing verbose args ({}), buf.len = {}",
                //     args.len(),
                //     buf.len()
                // );
                for arg in args {
                    let arg_bytes = &arg.as_bytes::<T>();
                    // println!("  arg_bytes: {:02X?}", arg_bytes);
                    buf.extend_from_slice(arg_bytes);
                }
            }
            PayloadContent::NonVerbose(msg_id, payload) => {
                // println!(
                //     "...Payload2::as_bytes, writing nonverbose, buf.len = {}",
                //     buf.len()
                // );
                #[allow(deprecated)]
                buf.put_u32::<T>(*msg_id);
                buf.extend_from_slice(&payload[..]);
            }
            PayloadContent::ControlMsg(ctrl_id, payload) => {
                // println!(
                //     "...Payload2::as_bytes, writing ControlType, buf.len = {}",
                //     buf.len()
                // );
                #[allow(deprecated)]
                buf.put_u8(ctrl_id.value());
                buf.extend_from_slice(&payload[..]);
            }
        }
        buf.to_vec()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Message {
    pub storage_header: Option<StorageHeader>,
    pub header: StandardHeader,
    pub extended_header: Option<ExtendedHeader>,
    pub payload: Payload2,
    #[serde(skip_serializing)]
    pub fibex_metadata: Option<Rc<FibexMetadata>>,
}
pub const DLT_COLUMN_SENTINAL: char = '\u{0004}';
pub const DLT_ARGUMENT_SENTINAL: char = '\u{0005}';
pub const DLT_NEWLINE_SENTINAL_SLICE: &[u8] = &[0x6];

lazy_static! {
    static ref DLT_NEWLINE_SENTINAL_STR: &'static str =
        unsafe { str::from_utf8_unchecked(DLT_NEWLINE_SENTINAL_SLICE) };
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExtendedHeaderConfig {
    pub message_type: MessageType,
    pub app_id: String,
    pub context_id: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MessageConfig {
    pub version: u8,
    pub counter: u8,
    pub endianness: Endianness,
    pub ecu_id: Option<String>,
    pub session_id: Option<u32>,
    pub timestamp: Option<u32>,
    pub payload: Payload2,
    pub extended_header_info: Option<ExtendedHeaderConfig>,
}

#[inline]
fn dbg_bytes(_index: usize, _name: &str, _bytes: &[u8]) {
    // println!(
    //     "writing {}[{}]: {} {:02X?}",
    //     _name,
    //     _index,
    //     _bytes.len(),
    //     _bytes
    // );
}
impl Message {
    pub fn new(
        conf: MessageConfig,
        fibex: Option<Rc<FibexMetadata>>,
        storage_header: Option<StorageHeader>,
    ) -> Self {
        // println!("--- Message::new, conf = {:?}", conf);
        // println!("--- Message::new, arg-cnt = {}", conf.payload.arg_count());
        let payload_length = if conf.endianness == Endianness::Big {
            conf.payload.as_bytes::<BigEndian>().len()
        } else {
            conf.payload.as_bytes::<LittleEndian>().len()
        } as u16;
        Message {
            header: StandardHeader {
                version: conf.version,
                endianness: conf.endianness,
                message_counter: conf.counter,
                ecu_id: conf.ecu_id,
                session_id: conf.session_id,
                timestamp: conf.timestamp,
                has_extended_header: conf.extended_header_info.is_some(),
                payload_length,
            },
            extended_header: match conf.extended_header_info {
                Some(ext_info) => Some(ExtendedHeader {
                    verbose: conf.payload.is_verbose(),
                    argument_count: conf.payload.arg_count(),
                    message_type: ext_info.message_type,
                    application_id: ext_info.app_id,
                    context_id: ext_info.context_id,
                }),
                None => None,
            },
            payload: conf.payload,
            fibex_metadata: fibex,
            storage_header,
        }
    }

    pub fn as_bytes(self: &Message) -> Vec<u8> {
        // println!(
        //     "message header overall_length: {}",
        //     self.header.overall_length()
        // );
        let mut capacity = self.header.overall_length() as usize;
        let mut buf = if let Some(storage_header) = &self.storage_header {
            capacity += STORAGE_HEADER_LENGTH;
            // println!("using capacity: {}", capacity);
            let mut b = BytesMut::with_capacity(capacity);
            // println!(
            //     "writing storage_header: {}",
            //     storage_header.as_bytes().len()
            // );
            b.extend_from_slice(&storage_header.as_bytes()[..]);
            b
        } else {
            // println!("using capacity: {}", capacity);
            BytesMut::with_capacity(capacity)
        };
        dbg_bytes(buf.len(), "header", &self.header.header_as_bytes()[..]);
        buf.extend_from_slice(&self.header.header_as_bytes()[..]);
        if let Some(ext_header) = &self.extended_header {
            let ext_header_bytes = ext_header.as_bytes();
            dbg_bytes(buf.len(), "ext_header", &ext_header_bytes[..]);
            buf.extend_from_slice(&ext_header_bytes[..]);
        }
        if self.header.endianness == Endianness::Big {
            let big_endian_payload = self.payload.as_bytes::<BigEndian>();
            dbg_bytes(buf.len(), "big endian payload", &big_endian_payload[..]);
            buf.extend_from_slice(&big_endian_payload[..]);
        } else {
            let little_endian_payload = self.payload.as_bytes::<LittleEndian>();
            dbg_bytes(
                buf.len(),
                "little endian payload",
                &little_endian_payload[..],
            );
            buf.extend_from_slice(&little_endian_payload[..]);
        }

        buf.to_vec()
    }
    fn write_app_id_context_id_and_message_type(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        match self.extended_header.as_ref() {
            Some(ext) => {
                write!(
                    f,
                    "{}{}{}{}{}{}",
                    ext.application_id,
                    DLT_COLUMN_SENTINAL,
                    ext.context_id,
                    DLT_COLUMN_SENTINAL,
                    ext.message_type,
                    DLT_COLUMN_SENTINAL,
                )?;
            }
            None => {
                write!(
                    f,
                    "-{}-{}-{}",
                    DLT_COLUMN_SENTINAL, DLT_COLUMN_SENTINAL, DLT_COLUMN_SENTINAL,
                )?;
            }
        };
        Ok(())
    }
}
/// will format dlt Message with those fields:
/// StorageHeader *************
///     - EColumn.DATETIME,
///     - EColumn.ECUID,
/// Version: EColumn.VERS,
/// SessionId: EColumn.SID,
/// message-count: EColumn.MCNT,
/// timestamp: EColumn.TMS,
/// EColumn.EID,
/// EColumn.APID,
/// EColumn.CTID,
/// EColumn.MSTP,
/// EColumn.PAYLOAD,
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(h) = &self.storage_header {
            write!(f, "{}", h)?;
        }
        write!(f, "{}", DLT_COLUMN_SENTINAL,)?;
        write!(f, "{}", self.header)?;
        write!(f, "{}", DLT_COLUMN_SENTINAL,)?;

        match &self.payload.payload_content {
            PayloadContent::Verbose(arguments) => {
                self.write_app_id_context_id_and_message_type(f)?;
                arguments
                    .iter()
                    .try_for_each(|arg| write!(f, "{}{}", DLT_ARGUMENT_SENTINAL, arg))
            }
            PayloadContent::NonVerbose(id, data) => self.format_nonverbose_data(*id, data, f),
            PayloadContent::ControlMsg(ctrl_id, _data) => {
                self.write_app_id_context_id_and_message_type(f)?;
                match SERVICE_ID_MAPPING.get(&ctrl_id.value()) {
                    Some((name, _desc)) => write!(f, "[{}]", name),
                    None => write!(f, "[Unknown CtrlCommand]"),
                }
            }
        }
    }
}

impl Message {
    fn format_nonverbose_data(&self, id: u32, data: &[u8], f: &mut fmt::Formatter) -> fmt::Result {
        let mut is_written = false;
        if let Some(fibex_metadata) = &self.fibex_metadata {
            let id_text = format!("ID_{}", id);
            let frame_metadata = if let Some(extended_header) = &self.extended_header {
                fibex_metadata.frame_map_with_key.get(&(
                    ContextId(extended_header.context_id.clone()),
                    ApplicationId(extended_header.application_id.clone()),
                    FrameId(id_text),
                )) // TODO: avoid cloning here (Cow or Borrow)
            } else {
                fibex_metadata.frame_map.get(&FrameId(id_text))
            };
            if let Some(frame_metadata) = frame_metadata {
                let FrameMetadata {
                    application_id,
                    context_id,
                    message_info,
                    ..
                } = &**frame_metadata;
                write!(
                    f,
                    "{}{}{}{}",
                    application_id
                        .as_ref()
                        .map(|id| &**id)
                        .or_else(|| self
                            .extended_header
                            .as_ref()
                            .map(|h| h.application_id.as_ref()))
                        .unwrap_or("-"),
                    DLT_COLUMN_SENTINAL,
                    context_id
                        .as_ref()
                        .map(|id| &**id)
                        .or_else(|| self.extended_header.as_ref().map(|h| h.context_id.as_ref()))
                        .unwrap_or("-"),
                    DLT_COLUMN_SENTINAL
                )?;
                if let Some(v) = message_info
                    .as_ref()
                    .and_then(|mi| MessageType::try_new_from_fibex_message_info(&*mi))
                {
                    write!(f, "{}", v)?;
                } else if let Some(message_type) =
                    self.extended_header.as_ref().map(|h| &h.message_type)
                {
                    write!(f, "{}", message_type)?;
                } else {
                    write!(f, "-")?;
                }
                write!(f, "{}", DLT_COLUMN_SENTINAL)?;
                let mut offset = 0;
                for pdu in &frame_metadata.pdus {
                    if let Some(description) = &pdu.description {
                        let arg = Argument {
                            type_info: TypeInfo {
                                kind: TypeInfoKind::StringType,
                                coding: StringCoding::UTF8,
                                has_trace_info: false,
                                has_variable_info: false,
                            },
                            name: None,
                            unit: None,
                            fixed_point: None,
                            value: Value::StringVal(description.to_string()),
                        };
                        write!(f, "{}{} ", DLT_ARGUMENT_SENTINAL, arg)?;
                    } else {
                        for signal_type in &pdu.signal_types {
                            let mut fixed_point = None;
                            let value = match signal_type.kind {
                                TypeInfoKind::StringType | TypeInfoKind::Raw => {
                                    if data.len() < offset + 2 {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    let length = if self.header.endianness == Endianness::Big {
                                        BigEndian::read_u16(&data[offset..offset + 2]) as usize
                                    } else {
                                        LittleEndian::read_u16(&data[offset..offset + 2]) as usize
                                    };
                                    offset += 2;
                                    if data.len() < offset + length {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    let v = match signal_type.kind {
                                        TypeInfoKind::StringType => Value::StringVal(
                                            String::from_utf8(
                                                data[offset..offset + length].to_vec(),
                                            )
                                            .map_err(|_| fmt::Error)?,
                                        ),
                                        TypeInfoKind::Raw => {
                                            Value::Raw(Vec::from(&data[offset..offset + length]))
                                        }
                                        _ => unreachable!(),
                                    };
                                    offset += length;
                                    v
                                }
                                TypeInfoKind::Bool => {
                                    offset += 1;
                                    if data.len() < offset {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    Value::Bool(data[offset - 1] != 0)
                                }
                                TypeInfoKind::Float(width) => {
                                    let length = width as usize / 8;
                                    if data.len() < offset + length {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    let v = if self.header.endianness == Endianness::Big {
                                        dlt_fint::<BigEndian>(width)(&data[offset..offset + length])
                                    } else {
                                        dlt_fint::<LittleEndian>(width)(
                                            &data[offset..offset + length],
                                        )
                                    }
                                    .map_err(|_| fmt::Error)?
                                    .1;
                                    offset += length;
                                    v
                                }
                                TypeInfoKind::Signed(length) => {
                                    let byte_length = length as usize / 8;
                                    if data.len() < offset + byte_length {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    let value_offset = &data[offset..];
                                    let (_, v) = if self.header.endianness == Endianness::Big {
                                        dlt_sint::<BigEndian>(length)(value_offset)
                                    } else {
                                        dlt_sint::<LittleEndian>(length)(value_offset)
                                    }
                                    .map_err(|_| fmt::Error)?;
                                    offset += byte_length;
                                    v
                                }
                                TypeInfoKind::SignedFixedPoint(length) => {
                                    let byte_length = length as usize / 8;
                                    if data.len() < offset + byte_length {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    let (value_offset, fp) =
                                        if self.header.endianness == Endianness::Big {
                                            dlt_fixed_point::<BigEndian>(
                                                &data[offset..offset + byte_length],
                                                length,
                                            )
                                        } else {
                                            dlt_fixed_point::<LittleEndian>(
                                                &data[offset..offset + byte_length],
                                                length,
                                            )
                                        }
                                        .map_err(|_| fmt::Error)?;
                                    fixed_point = Some(fp);
                                    let (_, v) =
                                        if self.header.endianness == Endianness::Big {
                                            dlt_sint::<BigEndian>(float_width_to_type_length(
                                                length,
                                            ))(
                                                value_offset
                                            )
                                        } else {
                                            dlt_sint::<LittleEndian>(float_width_to_type_length(
                                                length,
                                            ))(
                                                value_offset
                                            )
                                        }
                                        .map_err(|_| fmt::Error)?;
                                    offset += byte_length;
                                    v
                                }
                                TypeInfoKind::Unsigned(length) => {
                                    let byte_length = length as usize / 8;
                                    if data.len() < offset + byte_length {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    let value_offset = &data[offset..];
                                    let (_, v) = if self.header.endianness == Endianness::Big {
                                        dlt_uint::<BigEndian>(length)(value_offset)
                                    } else {
                                        dlt_uint::<LittleEndian>(length)(value_offset)
                                    }
                                    .map_err(|_| fmt::Error)?;
                                    offset += byte_length;
                                    v
                                }
                                TypeInfoKind::UnsignedFixedPoint(length) => {
                                    let byte_length = length as usize / 8;
                                    if data.len() < offset + byte_length {
                                        return fmt::Result::Err(fmt::Error);
                                    }
                                    let value_offset = {
                                        let (r, fp) =
                                            if self.header.endianness == Endianness::Big {
                                                dlt_fixed_point::<BigEndian>(
                                                    &data[offset..offset + byte_length],
                                                    length,
                                                )
                                            } else {
                                                dlt_fixed_point::<LittleEndian>(
                                                    &data[offset..offset + byte_length],
                                                    length,
                                                )
                                            }
                                            .map_err(|_| fmt::Error)?;
                                        fixed_point = Some(fp);
                                        r
                                    };
                                    let (_, v) =
                                        if self.header.endianness == Endianness::Big {
                                            dlt_uint::<BigEndian>(float_width_to_type_length(
                                                length,
                                            ))(
                                                value_offset
                                            )
                                        } else {
                                            dlt_uint::<LittleEndian>(float_width_to_type_length(
                                                length,
                                            ))(
                                                value_offset
                                            )
                                        }
                                        .map_err(|_| fmt::Error)?;
                                    offset += byte_length;
                                    v
                                }
                            };
                            let arg = Argument {
                                type_info: signal_type.clone(),
                                name: None,
                                unit: None,
                                fixed_point,
                                value,
                            };
                            write!(f, "{}{} ", DLT_ARGUMENT_SENTINAL, arg)?;
                        }
                    };
                    is_written = true;
                }
            } else {
                self.write_app_id_context_id_and_message_type(f)?;
            }
        } else {
            self.write_app_id_context_id_and_message_type(f)?;
        }
        if !is_written {
            let mut as_string = "- fibex missing -";
            if let Some(ext) = &self.extended_header {
                match &ext.message_type {
                    MessageType::Control(ct) => match ct {
                        ControlType::Request => as_string = "control request",
                        ControlType::Response => as_string = "control response",
                        ControlType::Unknown(_) => as_string = "unknown control",
                    },
                    MessageType::NetworkTrace(ntt) => match ntt {
                        NetworkTraceType::Ipc => as_string = "Ipc",
                        NetworkTraceType::Can => as_string = "Can",
                        NetworkTraceType::Flexray => as_string = "Flexray",
                        NetworkTraceType::Most => as_string = "Most",
                        NetworkTraceType::Ethernet => as_string = "Ethernet",
                        NetworkTraceType::Someip => as_string = "Someip",
                        NetworkTraceType::Invalid => as_string = "Invalid",
                        _ => as_string = "unknown network trace",
                    },
                    _ => (),
                }
            }

            f.write_str(
                &format!(
                    "{}[{}]{} {}",
                    DLT_ARGUMENT_SENTINAL, id, DLT_ARGUMENT_SENTINAL, as_string,
                )[..],
            )?;
        }
        Ok(())
    }
}

impl From<&LogLevel> for u8 {
    fn from(t: &LogLevel) -> Self {
        let mut res: u8 = 0;
        match t {
            LogLevel::Fatal => res |= 0x1 << 4,
            LogLevel::Error => res |= 0x2 << 4,
            LogLevel::Warn => res |= 0x3 << 4,
            LogLevel::Info => res |= 0x4 << 4,
            LogLevel::Debug => res |= 0x5 << 4,
            LogLevel::Verbose => res |= 0x6 << 4,
            LogLevel::Invalid(v) => res |= (v & 0b1111) << 4,
        }
        res
    }
}
// Convert dlt::LogLevel into log::Level
impl Into<log::Level> for LogLevel {
    fn into(self) -> log::Level {
        match self {
            LogLevel::Fatal | LogLevel::Error => log::Level::Error,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Info => log::Level::Info,
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Verbose => log::Level::Trace,
            LogLevel::Invalid(_) => log::Level::Trace,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            Value::Bool(value) => value.fmt(f),
            Value::U8(value) => value.fmt(f),
            Value::U16(value) => value.fmt(f),
            Value::U32(value) => value.fmt(f),
            Value::U64(value) => value.fmt(f),
            Value::U128(value) => value.fmt(f),
            Value::I8(value) => value.fmt(f),
            Value::I16(value) => value.fmt(f),
            Value::I32(value) => value.fmt(f),
            Value::I64(value) => value.fmt(f),
            Value::I128(value) => value.fmt(f),
            Value::F32(value) => value.fmt(f),
            Value::F64(value) => value.fmt(f),
            Value::StringVal(s) => write!(
                f,
                "{}",
                s.lines()
                    .collect::<Vec<&str>>()
                    .join(&DLT_NEWLINE_SENTINAL_STR)
            ),
            Value::Raw(value) => write!(f, "{:02X?}", value),
        }
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        if let Some(n) = &self.name {
            write!(f, "{}: ", n)?;
        }
        if let Some(u) = &self.unit {
            u.fmt(f)?;
        }
        if let Some(v) = self.to_real_value() {
            write!(f, "{}", v)?;
        } else {
            self.value.fmt(f)?;
        }

        Ok(())
    }
}

pub trait TryFrom<T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion. optionally supply an index to identify the
    /// dlt message in a log or stream
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

// StorageHeader
pub const STORAGE_HEADER_PATTERN_LENGTH: usize = 4;
pub const STORAGE_HEADER_LENGTH: usize = 16;

// Standard header
pub const WITH_EXTENDED_HEADER_FLAG: u8 = 1;
pub const BIG_ENDIAN_FLAG: u8 = 1 << 1;
pub const WITH_ECU_ID_FLAG: u8 = 1 << 2;
pub const WITH_SESSION_ID_FLAG: u8 = 1 << 3;
pub const WITH_TIMESTAMP_FLAG: u8 = 1 << 4;
pub const HEADER_MIN_LENGTH: u16 = 4;

// Verbose Mode

// Extended header
pub const VERBOSE_FLAG: u8 = 1;
pub const EXTENDED_HEADER_LENGTH: u16 = 10;

// Arguments
pub const TYPE_INFO_LENGTH: usize = 4;
pub const TYPE_INFO_BOOL_FLAG: u32 = 1 << 4;
pub const TYPE_INFO_SINT_FLAG: u32 = 1 << 5;
pub const TYPE_INFO_UINT_FLAG: u32 = 1 << 6;
pub const TYPE_INFO_FLOAT_FLAG: u32 = 1 << 7;
pub const _TYPE_INFO_ARRAY_FLAG: u32 = 1 << 8;
pub const TYPE_INFO_STRING_FLAG: u32 = 1 << 9;
pub const TYPE_INFO_RAW_FLAG: u32 = 1 << 10;
pub const TYPE_INFO_VARIABLE_INFO: u32 = 1 << 11;
pub const TYPE_INFO_FIXED_POINT_FLAG: u32 = 1 << 12;
pub const TYPE_INFO_TRACE_INFO_FLAG: u32 = 1 << 13;
#[allow(dead_code)]
pub const TYPE_INFO_STRUCT_FLAG: u32 = 1 << 14;

// TODO use header struct not u8
pub fn calculate_standard_header_length(header_type: u8) -> u16 {
    let mut length = HEADER_MIN_LENGTH;
    if (header_type & WITH_ECU_ID_FLAG) != 0 {
        length += 4;
    }
    if (header_type & WITH_SESSION_ID_FLAG) != 0 {
        length += 4;
    }
    if (header_type & WITH_TIMESTAMP_FLAG) != 0 {
        length += 4;
    }
    length
}

// TODO use header struct not u8
pub fn calculate_all_headers_length(header_type: u8) -> u16 {
    let mut length = calculate_standard_header_length(header_type);
    if (header_type & WITH_EXTENDED_HEADER_FLAG) != 0 {
        length += EXTENDED_HEADER_LENGTH;
    }
    length
}

pub fn zero_terminated_string(raw: &[u8]) -> Result<String, Error> {
    let nul_range_end = raw
        .iter()
        .position(|&c| c == b'\0')
        .unwrap_or_else(|| raw.len()); // default to length if no `\0` present
    str::from_utf8(&raw[0..nul_range_end])
        .map(|v| v.to_owned())
        .map_err(|e| {
            report_warning(format!("Invalid zero_terminated_string: {}", e));
            Error::new(io::ErrorKind::Other, e)
        })
}

pub const LEVEL_FATAL: u8 = 0x1;
pub const LEVEL_ERROR: u8 = 0x2;
pub const LEVEL_WARN: u8 = 0x3;
pub const LEVEL_INFO: u8 = 0x4;
pub const LEVEL_DEBUG: u8 = 0x5;
pub const LEVEL_VERBOSE: u8 = 0x6;

pub fn u8_to_log_level(v: u8) -> Option<LogLevel> {
    match v {
        LEVEL_FATAL => Some(LogLevel::Fatal),
        LEVEL_ERROR => Some(LogLevel::Error),
        LEVEL_WARN => Some(LogLevel::Warn),
        LEVEL_INFO => Some(LogLevel::Info),
        LEVEL_DEBUG => Some(LogLevel::Debug),
        LEVEL_VERBOSE => Some(LogLevel::Verbose),
        _ => None,
    }
}
impl TryFrom<u8> for LogLevel {
    type Error = Error;
    fn try_from(message_info: u8) -> Result<LogLevel, Error> {
        let raw = message_info >> 4;
        let level = u8_to_log_level(raw);
        match level {
            Some(n) => Ok(n),
            None => Ok(LogLevel::Invalid(raw)),
        }
    }
}

impl From<&ApplicationTraceType> for u8 {
    fn from(t: &ApplicationTraceType) -> Self {
        match t {
            ApplicationTraceType::Variable => 0x1 << 4,
            ApplicationTraceType::FunctionIn => 0x2 << 4,
            ApplicationTraceType::FunctionOut => 0x3 << 4,
            ApplicationTraceType::State => 0x4 << 4,
            ApplicationTraceType::Vfb => 0x5 << 4,
            ApplicationTraceType::Invalid(n) => n << 4,
        }
    }
}

impl TryFrom<u8> for ApplicationTraceType {
    type Error = Error;
    fn try_from(message_info: u8) -> Result<ApplicationTraceType, Error> {
        match message_info >> 4 {
            1 => Ok(ApplicationTraceType::Variable),
            2 => Ok(ApplicationTraceType::FunctionIn),
            3 => Ok(ApplicationTraceType::FunctionOut),
            4 => Ok(ApplicationTraceType::State),
            5 => Ok(ApplicationTraceType::Vfb),
            n => Ok(ApplicationTraceType::Invalid(n)),
        }
    }
}

impl From<&NetworkTraceType> for u8 {
    fn from(t: &NetworkTraceType) -> Self {
        match t {
            NetworkTraceType::Invalid => 0x0 << 4,
            NetworkTraceType::Ipc => 0x1 << 4,
            NetworkTraceType::Can => 0x2 << 4,
            NetworkTraceType::Flexray => 0x3 << 4,
            NetworkTraceType::Most => 0x4 << 4,
            NetworkTraceType::Ethernet => 0x5 << 4,
            NetworkTraceType::Someip => 0x6 << 4,
            NetworkTraceType::UserDefined(v) => v << 4,
        }
    }
}

impl TryFrom<u8> for NetworkTraceType {
    type Error = Error;
    fn try_from(message_info: u8) -> Result<NetworkTraceType, Error> {
        match message_info >> 4 {
            0 => Ok(NetworkTraceType::Invalid),
            1 => Ok(NetworkTraceType::Ipc),
            2 => Ok(NetworkTraceType::Can),
            3 => Ok(NetworkTraceType::Flexray),
            4 => Ok(NetworkTraceType::Most),
            5 => Ok(NetworkTraceType::Ethernet),
            6 => Ok(NetworkTraceType::Someip),
            n => Ok(NetworkTraceType::UserDefined(n)),
        }
    }
}

impl From<&ControlType> for u8 {
    fn from(t: &ControlType) -> Self {
        let mut res: u8 = 0;
        match t {
            ControlType::Request => res |= 0x1 << 4,
            ControlType::Response => res |= 0x2 << 4,
            ControlType::Unknown(n) => res |= n << 4,
        }
        res
    }
}
impl TryFrom<u8> for ControlType {
    type Error = Error;
    fn try_from(message_info: u8) -> Result<ControlType, Error> {
        match message_info >> 4 {
            1 => Ok(ControlType::Request),
            2 => Ok(ControlType::Response),
            n => Ok(ControlType::Unknown(n)),
        }
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            MessageType::ApplicationTrace(app_type) => app_type.fmt(f),
            MessageType::Control(c) => c.fmt(f),
            MessageType::Log(log_level) => log_level.fmt(f),
            MessageType::NetworkTrace(trace_type) => trace_type.fmt(f),
            MessageType::Unknown(v) => write!(f, "Unkown MSTP {:?}", v),
        }
    }
}
impl From<&MessageType> for u8 {
    fn from(t: &MessageType) -> Self {
        match t {
            MessageType::Log(x) =>
            /* 0x0 << 1 |*/
            {
                u8::from(x)
            }
            MessageType::ApplicationTrace(x) => 0x1 << 1 | u8::from(x),
            MessageType::NetworkTrace(x) => 0x2 << 1 | u8::from(x),
            MessageType::Control(x) => 0x3 << 1 | u8::from(x),
            MessageType::Unknown((mstp, mtin)) => mstp << 1 | mtin << 4,
        }
    }
}
impl TryFrom<u8> for MessageType {
    type Error = Error;
    fn try_from(message_info: u8) -> Result<MessageType, Error> {
        match (message_info >> 1) & 0b111 {
            DLT_TYPE_LOG => Ok(MessageType::Log(LogLevel::try_from(message_info)?)),
            DLT_TYPE_APP_TRACE => Ok(MessageType::ApplicationTrace(
                ApplicationTraceType::try_from(message_info)?,
            )),
            DLT_TYPE_NW_TRACE => Ok(MessageType::NetworkTrace(NetworkTraceType::try_from(
                message_info,
            )?)),
            DLT_TYPE_CONTROL => Ok(MessageType::Control(ControlType::try_from(message_info)?)),
            v => Ok(MessageType::Unknown((v, (message_info >> 4) & 0b1111))),
        }
    }
}
