// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::io;

use crate::varint;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    TinyInt(i8),
    SmallInt(i16),
    Integer(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Date(i32),
    Time(i32),
    String(Vec<u8>),
    Bytes(Vec<u8>),
    DecimalCompact(i64),
    DecimalLarge(Vec<u8>),
    TimestampMillis(i64),
    TimestampMicros(i64),
    TimestampNanos { millis: i64, nanos_of_milli: i32 },
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn to_be_bytes(&self) -> Vec<u8> {
        match self {
            Value::Null => Vec::new(),
            Value::Boolean(v) => vec![if *v { 1 } else { 0 }],
            Value::TinyInt(v) => vec![*v as u8],
            Value::SmallInt(v) => v.to_be_bytes().to_vec(),
            Value::Integer(v) | Value::Date(v) | Value::Time(v) => v.to_be_bytes().to_vec(),
            Value::BigInt(v)
            | Value::DecimalCompact(v)
            | Value::TimestampMillis(v)
            | Value::TimestampMicros(v) => v.to_be_bytes().to_vec(),
            Value::Float(v) => v.to_bits().to_be_bytes().to_vec(),
            Value::Double(v) => v.to_bits().to_be_bytes().to_vec(),
            Value::TimestampNanos {
                millis,
                nanos_of_milli,
            } => {
                let mut buf = millis.to_be_bytes().to_vec();
                buf.extend_from_slice(&nanos_of_milli.to_be_bytes());
                buf
            }
            Value::String(b) | Value::Bytes(b) | Value::DecimalLarge(b) => b.clone(),
        }
    }
}

fn type_mismatch(expected: &str, value: &Value) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("type mismatch: expected {}, got {:?}", expected, value),
    )
}

pub fn write_fixed(buf: &mut Vec<u8>, value: &Value, width: i32) -> io::Result<()> {
    match width {
        1 => match value {
            Value::Boolean(v) => buf.push(if *v { 1 } else { 0 }),
            Value::TinyInt(v) => buf.push(*v as u8),
            _ => return Err(type_mismatch("Boolean or TinyInt", value)),
        },
        2 => match value {
            Value::SmallInt(v) => buf.extend_from_slice(&v.to_be_bytes()),
            _ => return Err(type_mismatch("SmallInt", value)),
        },
        4 => match value {
            Value::Integer(v) | Value::Date(v) | Value::Time(v) => {
                buf.extend_from_slice(&v.to_be_bytes());
            }
            Value::Float(v) => {
                buf.extend_from_slice(&v.to_bits().to_be_bytes());
            }
            _ => return Err(type_mismatch("Integer/Date/Time/Float", value)),
        },
        8 => match value {
            Value::BigInt(v)
            | Value::DecimalCompact(v)
            | Value::TimestampMillis(v)
            | Value::TimestampMicros(v) => {
                buf.extend_from_slice(&v.to_be_bytes());
            }
            Value::Double(v) => {
                buf.extend_from_slice(&v.to_bits().to_be_bytes());
            }
            _ => return Err(type_mismatch("BigInt/Decimal/Timestamp/Double", value)),
        },
        12 => match value {
            Value::TimestampNanos {
                millis,
                nanos_of_milli,
            } => {
                buf.extend_from_slice(&millis.to_be_bytes());
                buf.extend_from_slice(&nanos_of_milli.to_be_bytes());
            }
            _ => return Err(type_mismatch("TimestampNanos", value)),
        },
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported fixed width {}", width),
            ))
        }
    }
    Ok(())
}

pub fn write_variable(buf: &mut Vec<u8>, value: &Value) -> io::Result<()> {
    match value {
        Value::String(bytes) | Value::Bytes(bytes) | Value::DecimalLarge(bytes) => {
            varint::encode(buf, bytes.len() as u32);
            buf.extend_from_slice(bytes);
            Ok(())
        }
        _ => Err(type_mismatch("String/Bytes/DecimalLarge", value)),
    }
}

pub fn extract_fixed_key(buf: &[u8], pos: usize, width: i32) -> u64 {
    match width {
        1 => buf[pos] as u64,
        2 => u16::from_be_bytes([buf[pos], buf[pos + 1]]) as u64,
        4 => u32::from_be_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]) as u64,
        8 => u64::from_be_bytes([
            buf[pos],
            buf[pos + 1],
            buf[pos + 2],
            buf[pos + 3],
            buf[pos + 4],
            buf[pos + 5],
            buf[pos + 6],
            buf[pos + 7],
        ]),
        _ => 0,
    }
}
