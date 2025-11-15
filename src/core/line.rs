use chrono::{Datelike, Duration, NaiveDateTime, Timelike};
use dbc_editor::types::database::{DatabaseDBC, MessageKey};
use smallvec::SmallVec;

use crate::types::frame::{Direction, Frame, FrameType};
use crate::types::keys::FrameKey;
use crate::types::log::{ChannelType, Log};

const MAX_CAN_PAYLOAD: usize = 64;

pub struct LineParser {
    data_buf: String,
    payload_buf: SmallVec<[u8; MAX_CAN_PAYLOAD]>,
}

impl LineParser {
    pub fn new() -> Self {
        Self {
            data_buf: String::with_capacity(24),
            payload_buf: SmallVec::new(),
        }
    }

    // Example:
    // 0.016728 1 17334410x Rx d 8 3E 42 03 00 39 00 03 01
    // 0.016728 1 17334410x Rx Name ECU d 8 3E 42 03 00 39 00 03 01
    pub fn parse(&mut self, line: &str, log: &mut Log) {
        // split line by whitespaces (ASCII only, faster than Unicode-aware split)
        let mut it = line.split_ascii_whitespace();

        // Build the frame
        let mut frame: Frame = Frame::default();

        // Timestamp
        let ts_tok: &str = match it.next() {
            Some(v) => v,
            None => return,
        };
        let timestamp: f64 = match ts_tok.parse() {
            Ok(v) => v,
            Err(_) => return,
        };

        // Channel
        let ch_tok: &str = match it.next() {
            Some(v) => v,
            None => return,
        };
        let channel: u8 = match ch_tok.parse::<u8>() {
            Ok(v) => v,
            Err(_) => return,
        };

        frame.timestamp = timestamp;
        frame.channel = channel;
        match log.channel_map.get(&channel) {
            Some(ch_info) => match ch_info.tipo {
                ChannelType::Can => frame.ftype = FrameType::Can,
                ChannelType::Ethernet => frame.ftype = FrameType::Eth,
            },
            None => return,
        }

        // -------- Can Frame parsing ----------- //
        if frame.ftype == FrameType::Can {
            // id token (keep original string for the frame)
            let id_tok: &str = match it.next() {
                Some(v) => v,
                None => {
                    frame.ftype = FrameType::ErrorFrame;
                    let frame_key: FrameKey = log.frames.insert(frame);
                    log.frame_by_file_order.push(frame_key);
                    return;
                }
            };
            // Message Id e Id_Hex
            let id: u32 = match parse_id_u32(id_tok) {
                Some(v) => v,
                None => return,
            };

            frame.id = id;
            frame.id_hex = id_tok.to_string();

            // Direction
            match it.next() {
                Some(v) => match v {
                    "Tx" => frame.direction = Direction::Tx,
                    "Rx" => frame.direction = Direction::Rx,
                    _ => {
                        frame.ftype = FrameType::ErrorFrame;
                        let frame_key: FrameKey = log.frames.insert(frame);
                        log.frame_by_file_order.push(frame_key);
                        return;
                    }
                },
                None => {
                    frame.ftype = FrameType::ErrorFrame;
                    let frame_key: FrameKey = log.frames.insert(frame);
                    log.frame_by_file_order.push(frame_key);
                    return;
                }
            };

            // Scan forward to 'd' or 'D', then read byte length and payload tokens
            let mut after_d: Option<&str> = None;
            while let Some(tok) = it.next() {
                if tok == "d" || tok == "D" {
                    after_d = it.next(); // next is byte length
                    break;
                }
            }

            // Byte Length
            frame.byte_length = match after_d.and_then(|s| s.parse().ok()) {
                Some(v) => v,
                None => {
                    frame.ftype = FrameType::ErrorFrame;
                    let frame_key: FrameKey = log.frames.insert(frame);
                    log.frame_by_file_order.push(frame_key);
                    return;
                }
            };

            // Collect N payload tokens into a single space-separated String while decoding bytes
            let payload_len: usize = frame.byte_length as usize;
            self.data_buf.clear();
            let needed_chars: usize = payload_len.saturating_mul(3);
            if self.data_buf.capacity() < needed_chars {
                self.data_buf
                    .reserve(needed_chars - self.data_buf.capacity());
            }
            self.payload_buf.clear();
            if self.payload_buf.capacity() < payload_len {
                self.payload_buf
                    .reserve(payload_len - self.payload_buf.capacity());
            }

            for i in 0..payload_len {
                let tok = match it.next() {
                    Some(v) => v,
                    None => return, // malformed: not enough data bytes
                };
                if i != 0 {
                    self.data_buf.push(' ');
                }
                self.data_buf.push_str(tok);

                let byte = match u8::from_str_radix(tok, 16) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                self.payload_buf.push(byte);
            }

            std::mem::swap(&mut frame.data, &mut self.data_buf);

            // absolute time of the single CanFrame
            frame.absolute_time = if let Some(start_time) = log.absolute_time.value {
                let delta_ms: i64 = (timestamp * 1000.0).round() as i64;
                let abs_time_value: NaiveDateTime = start_time + Duration::milliseconds(delta_ms);
                format_datetime_ymdhms_millis(abs_time_value)
            } else {
                seconds_to_hms_string(timestamp)
            };

            // If a DBC is available for this channel, try to decode
            if let Some(dbc) = log.get_database_by_channel(channel)
                && let Some(msg_key) = resolve_msg_key_for_id(dbc, id)
                && let Some(msg) = dbc.get_message_by_key(msg_key)
            {
                frame.msg_key = msg_key;
                if let Some(&node_key) = msg.sender_nodes.first() {
                    frame.tx_node_key = node_key;
                }
                frame.sig_keys = msg.signals.clone();
            }

            if let Some(dbc) = log.get_mut_database_by_channel(channel) {
                let payload_bytes: &[u8] = &self.payload_buf;
                for &sig_key in frame.sig_keys.iter() {
                    if let Some(signal) = dbc.get_sig_by_key_mut(sig_key) {
                        let raw: i64 = signal.extract_raw_i64(&payload_bytes);
                        let value: f64 = (raw as f64) * signal.factor + signal.offset;

                        // Append a point to the corresponding SignalDBC time series
                        signal.raws.push((timestamp, raw));
                        signal.values.push((timestamp, value));
                    }
                }
            };

            // Inserisci il frame nella lista una volta terminata la decodifica
            let frame_key: FrameKey = log.frames.insert(frame);
            log.frame_by_file_order.push(frame_key);
        } // if frame.ftype == FrameType::Can
    }
}

fn seconds_to_hms_string(seconds: f64) -> String {
    let total_millis: u32 = (seconds * 1000.0).round() as u32;

    let hours: u32 = total_millis / 3_600_000;
    let minutes: u32 = (total_millis % 3_600_000) / 60_000;
    let secs: u32 = (total_millis % 60_000) / 1000;
    let millis: u32 = total_millis % 1000;

    format!(
        "2025-01-01 {:02}:{:02}:{:02}.{:03}",
        hours, minutes, secs, millis
    )
}

// Fast formatter: YYYY-MM-DD HH:MM:SS.mmm
fn format_datetime_ymdhms_millis(dt: NaiveDateTime) -> String {
    let year: i32 = dt.year();
    let month: u32 = dt.month();
    let day: u32 = dt.day();
    let hour: u32 = dt.hour();
    let minute: u32 = dt.minute();
    let second: u32 = dt.second();
    let millis: u32 = dt.and_utc().timestamp_subsec_millis();

    let mut out = String::with_capacity(23);
    out.push_str(&year.to_string());
    out.push('-');
    push_2(&mut out, month);
    out.push('-');
    push_2(&mut out, day);
    out.push(' ');
    push_2(&mut out, hour);
    out.push(':');
    push_2(&mut out, minute);
    out.push(':');
    push_2(&mut out, second);
    out.push('.');
    push_3(&mut out, millis);
    out
}

#[inline]
fn push_2(buf: &mut String, v: u32) {
    let d1 = ((v / 10) % 10) as u8 + b'0';
    let d2 = (v % 10) as u8 + b'0';
    buf.push(d1 as char);
    buf.push(d2 as char);
}

#[inline]
fn push_3(buf: &mut String, v: u32) {
    let d1 = ((v / 100) % 10) as u8 + b'0';
    let d2 = ((v / 10) % 10) as u8 + b'0';
    let d3 = (v % 10) as u8 + b'0';
    buf.push(d1 as char);
    buf.push(d2 as char);
    buf.push(d3 as char);
}

/// Normalize an ASC id token like "17334410x" or "12AB" to "0x17334410" / "0x12AB".
fn parse_id_u32(id_token: &str) -> Option<u32> {
    // strip trailing x/X (extended id marker)
    let s: &str = id_token.trim_end_matches(['x', 'X']);
    let s: &str = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        hex
    } else {
        s
    };

    // Prefer hexadecimal interpretation (Vector default). If it overflows or fails, retry as decimal.
    match u32::from_str_radix(s, 16) {
        Ok(value) => Some(value),
        Err(_) => s.parse::<u32>().ok(),
    }
}

const CAN_STD_MAX_ID: u32 = 0x7FF;
const CAN_EFF_FLAG: u32 = 0x8000_0000;

fn resolve_msg_key_for_id(dbc: &DatabaseDBC, id: u32) -> Option<MessageKey> {
    dbc.get_msg_key_by_id(id).or_else(|| {
        if id > CAN_STD_MAX_ID {
            dbc.get_msg_key_by_id(id | CAN_EFF_FLAG)
        } else {
            None
        }
    })
}
