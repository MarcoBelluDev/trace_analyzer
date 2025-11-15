use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use ordered_float::OrderedFloat;

use crate::core;
use crate::core::line::LineParser;
use crate::types::errors::AscParseError;
use crate::types::frame::{Direction, Frame, FrameType};
use crate::types::keys::FrameKey;
use crate::types::log::Log;

/// Parses a Vector ASCII trace (`.asc`) file and builds a `Log`.
pub fn from_asc_file(path: &str, log: &mut Log) -> Result<(), AscParseError> {
    // clear frames
    log.clear_frames();

    // check if provided file has .asc format
    if !path.ends_with(".asc") {
        return Err(AscParseError::InvalidExtension {
            path: path.to_string(),
        });
    }

    // temporary registry: (name, channel) -> Signal index
    let mut found_abs_time: bool = false;

    let path_owned: String = path.to_string();
    let mut reader: BufReader<File> = match File::open(path) {
        Ok(file) => BufReader::new(file),
        Err(source) => {
            return Err(AscParseError::OpenFile {
                path: path_owned.clone(),
                source,
            });
        }
    };

    let mut line_parser: LineParser = LineParser::new();

    // read .asc file line by line reusing the same buffer
    let mut line: String = String::new();
    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|source| AscParseError::Read {
                path: path_owned.clone(),
                source,
            })?;
        if bytes_read == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if !found_abs_time && let Some(time) = core::abs_time::from_line(trimmed) {
            log.absolute_time = time;
            found_abs_time = true;
            continue; // skip abs_time check for rest of the line
        }
        line_parser.parse(trimmed, log);
    }

    // ---- Sorting ---- //
    let base_keys: &[FrameKey] = log.frame_by_file_order.as_slice();
    let order_index: HashMap<FrameKey, usize> = base_keys
        .iter()
        .enumerate()
        .map(|(idx, key)| (*key, idx))
        .collect();
    let frames = &log.frames;
    let channel_map = &log.channel_map;
    let fallback_index =
        |key: FrameKey| -> usize { order_index.get(&key).copied().unwrap_or(usize::MAX) };
    let refill = |target: &mut Vec<FrameKey>, source: &[FrameKey]| {
        target.clear();
        target.extend_from_slice(source);
    };

    let mut last_by_id_channel: HashMap<(u32, u8), FrameKey> = HashMap::new();
    for &key in base_keys {
        if let Some(frame) = frames.get(key)
            && frame.ftype == FrameType::Can
        {
            last_by_id_channel.insert((frame.id, frame.channel), key);
        }
    }
    let mut id_chn_keys: Vec<FrameKey> = last_by_id_channel.values().copied().collect();
    id_chn_keys.sort_by_key(|key| fallback_index(*key));

    let direction_rank = |dir: &Direction| match dir {
        Direction::Rx => 0_u8,
        Direction::Tx => 1_u8,
    };

    let protocol_rank = |frame: &Frame| -> u8 { if frame.byte_length <= 8 { 0 } else { 1 } };

    let sort_by_timestamp = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            match frames.get(key) {
                Some(frame) => (0_u8, OrderedFloat(frame.timestamp), fallback),
                None => (1_u8, OrderedFloat(0.0), fallback),
            }
        });
    };

    let sort_by_channel = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            match frames.get(key) {
                Some(frame) => (0_u8, frame.channel, OrderedFloat(frame.timestamp), fallback),
                None => (1_u8, u8::MAX, OrderedFloat(0.0), fallback),
            }
        });
    };

    let sort_by_direction = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            match frames.get(key) {
                Some(frame) => (
                    0_u8,
                    direction_rank(&frame.direction),
                    OrderedFloat(frame.timestamp),
                    fallback,
                ),
                None => (1_u8, u8::MAX, OrderedFloat(0.0), fallback),
            }
        });
    };

    let sort_by_can_msg_name = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            let (rank, name) = frames
                .get(key)
                .and_then(|frame| {
                    channel_map
                        .get(&frame.channel)
                        .and_then(|info| info.database.as_ref())
                        .and_then(|db| db.get_message_by_key(frame.msg_key))
                        .map(|msg| msg.name.as_str())
                })
                .map(|name| (0_u8, name))
                .unwrap_or((1_u8, ""));
            (rank, name, fallback)
        });
    };

    let sort_by_can_msg_id = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            match frames.get(key) {
                Some(frame) => (0_u8, frame.id, fallback),
                None => (1_u8, u32::MAX, fallback),
            }
        });
    };

    let sort_by_can_dlc = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            match frames.get(key) {
                Some(frame) => (0_u8, frame.byte_length, fallback),
                None => (1_u8, u16::MAX, fallback),
            }
        });
    };

    let sort_by_can_protocol = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            match frames.get(key) {
                Some(frame) => (0_u8, protocol_rank(frame), frame.byte_length, fallback),
                None => (1_u8, u8::MAX, u16::MAX, fallback),
            }
        });
    };

    let sort_by_can_sender_node = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            let (rank, name) = frames
                .get(key)
                .and_then(|frame| {
                    channel_map
                        .get(&frame.channel)
                        .and_then(|info| info.database.as_ref())
                        .and_then(|db| db.get_node_by_key(frame.tx_node_key))
                        .map(|node| node.name.as_str())
                })
                .map(|name| (0_u8, name))
                .unwrap_or((1_u8, ""));
            (rank, name, fallback)
        });
    };

    let sort_by_can_data = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            match frames.get(key) {
                Some(frame) => (0_u8, frame.data.as_str(), fallback),
                None => (1_u8, "", fallback),
            }
        });
    };

    let sort_by_can_comment = |vec: &mut Vec<FrameKey>| {
        vec.sort_by_key(|key| {
            let key = *key;
            let fallback = fallback_index(key);
            let (rank, comment) = frames
                .get(key)
                .and_then(|frame| {
                    channel_map
                        .get(&frame.channel)
                        .and_then(|info| info.database.as_ref())
                        .and_then(|db| db.get_message_by_key(frame.msg_key))
                        .map(|msg| msg.comment.as_str())
                })
                .map(|comment| (0_u8, comment))
                .unwrap_or((1_u8, ""));
            (rank, comment, fallback)
        });
    };

    refill(&mut log.frame_by_timestamp, base_keys);
    sort_by_timestamp(&mut log.frame_by_timestamp);
    refill(&mut log.id_chn_by_timestamp, &id_chn_keys);
    sort_by_timestamp(&mut log.id_chn_by_timestamp);

    refill(&mut log.frame_by_channel, base_keys);
    sort_by_channel(&mut log.frame_by_channel);
    refill(&mut log.id_chn_by_channel, &id_chn_keys);
    sort_by_channel(&mut log.id_chn_by_channel);

    refill(&mut log.frame_by_direction, base_keys);
    sort_by_direction(&mut log.frame_by_direction);
    refill(&mut log.id_chn_by_direction, &id_chn_keys);
    sort_by_direction(&mut log.id_chn_by_direction);

    let can_keys: Vec<FrameKey> = base_keys
        .iter()
        .copied()
        .filter(|key| matches!(frames.get(*key), Some(frame) if frame.ftype == FrameType::Can))
        .collect();

    refill(&mut log.frame_by_can_msg_name, &can_keys);
    sort_by_can_msg_name(&mut log.frame_by_can_msg_name);
    refill(&mut log.id_chn_by_can_msg_name, &id_chn_keys);
    sort_by_can_msg_name(&mut log.id_chn_by_can_msg_name);

    refill(&mut log.frame_by_can_msg_id, &can_keys);
    sort_by_can_msg_id(&mut log.frame_by_can_msg_id);
    refill(&mut log.id_chn_by_can_msg_id, &id_chn_keys);
    sort_by_can_msg_id(&mut log.id_chn_by_can_msg_id);

    refill(&mut log.frame_by_can_dlc, &can_keys);
    sort_by_can_dlc(&mut log.frame_by_can_dlc);
    refill(&mut log.id_chn_by_can_dlc, &id_chn_keys);
    sort_by_can_dlc(&mut log.id_chn_by_can_dlc);

    refill(&mut log.frame_by_can_protocol, &can_keys);
    sort_by_can_protocol(&mut log.frame_by_can_protocol);
    refill(&mut log.id_chn_by_can_protocol, &id_chn_keys);
    sort_by_can_protocol(&mut log.id_chn_by_can_protocol);

    refill(&mut log.frame_by_can_sender_node, &can_keys);
    sort_by_can_sender_node(&mut log.frame_by_can_sender_node);
    refill(&mut log.id_chn_by_can_sender_node, &id_chn_keys);
    sort_by_can_sender_node(&mut log.id_chn_by_can_sender_node);

    refill(&mut log.frame_by_can_data, &can_keys);
    sort_by_can_data(&mut log.frame_by_can_data);
    refill(&mut log.id_chn_by_can_data, &id_chn_keys);
    sort_by_can_data(&mut log.id_chn_by_can_data);

    refill(&mut log.frame_by_can_comment, &can_keys);
    sort_by_can_comment(&mut log.frame_by_can_comment);
    refill(&mut log.id_chn_by_can_comment, &id_chn_keys);
    sort_by_can_comment(&mut log.id_chn_by_can_comment);

    Ok(())
}
