use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::core;
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
        core::line::parse(trimmed, log);
    }

    // ---- Sorting ---- //
    let base_keys: Vec<FrameKey> = log.frame_by_file_order.clone();
    let order_index: HashMap<FrameKey, usize> = base_keys
        .iter()
        .enumerate()
        .map(|(idx, key)| (*key, idx))
        .collect();
    let frames = &log.frames;
    let channel_map = &log.channel_map;
    let cmp_f64 = |lhs: f64, rhs: f64| lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal);
    let key_position = |key: &FrameKey| order_index.get(key).copied().unwrap_or(usize::MAX);
    let fallback_cmp =
        |lhs: &FrameKey, rhs: &FrameKey| -> Ordering { key_position(lhs).cmp(&key_position(rhs)) };
    let refill = |target: &mut Vec<FrameKey>, source: &[FrameKey]| {
        target.clear();
        target.extend(source.iter().copied());
    };

    refill(&mut log.frame_by_timestamp, &base_keys);
    log.frame_by_timestamp
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let ts_ord: Ordering = cmp_f64(fa.timestamp, fb.timestamp);
                if ts_ord == Ordering::Equal {
                    fallback_cmp(a, b)
                } else {
                    ts_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    refill(&mut log.frame_by_channel, &base_keys);
    log.frame_by_channel
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let ch_ord: Ordering = fa.channel.cmp(&fb.channel);
                if ch_ord == Ordering::Equal {
                    let ts_ord: Ordering = cmp_f64(fa.timestamp, fb.timestamp);
                    if ts_ord == Ordering::Equal {
                        fallback_cmp(a, b)
                    } else {
                        ts_ord
                    }
                } else {
                    ch_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    refill(&mut log.frame_by_direction, &base_keys);
    log.frame_by_direction.sort_by(|a, b| {
        let rank = |dir: &Direction| match dir {
            Direction::Rx => 0_u8,
            Direction::Tx => 1_u8,
        };

        match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let dir_ord: Ordering = rank(&fa.direction).cmp(&rank(&fb.direction));
                if dir_ord == Ordering::Equal {
                    let ts_ord: Ordering = cmp_f64(fa.timestamp, fb.timestamp);
                    if ts_ord == Ordering::Equal {
                        fallback_cmp(a, b)
                    } else {
                        ts_ord
                    }
                } else {
                    dir_ord
                }
            }
            _ => fallback_cmp(a, b),
        }
    });

    let can_keys: Vec<FrameKey> = base_keys
        .iter()
        .copied()
        .filter(|key| matches!(frames.get(*key), Some(frame) if frame.ftype == FrameType::Can))
        .collect();

    refill(&mut log.frame_by_can_msg_name, &can_keys);
    log.frame_by_can_msg_name
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let name_a: &str = channel_map
                    .get(&fa.channel)
                    .and_then(|info| info.database.as_ref())
                    .and_then(|db| db.get_message_by_key(fa.msg_key))
                    .map(|msg| msg.name.as_str())
                    .unwrap_or("");
                let name_b: &str = channel_map
                    .get(&fb.channel)
                    .and_then(|info| info.database.as_ref())
                    .and_then(|db| db.get_message_by_key(fb.msg_key))
                    .map(|msg| msg.name.as_str())
                    .unwrap_or("");
                let name_ord: Ordering = name_a.cmp(name_b);
                if name_ord == Ordering::Equal {
                    fallback_cmp(a, b)
                } else {
                    name_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    refill(&mut log.frame_by_can_msg_id, &can_keys);
    log.frame_by_can_msg_id
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let id_ord: Ordering = fa.id.cmp(&fb.id);
                if id_ord == Ordering::Equal {
                    fallback_cmp(a, b)
                } else {
                    id_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    refill(&mut log.frame_by_can_dlc, &can_keys);
    log.frame_by_can_dlc
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let dlc_ord: Ordering = fa.byte_length.cmp(&fb.byte_length);
                if dlc_ord == Ordering::Equal {
                    fallback_cmp(a, b)
                } else {
                    dlc_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    refill(&mut log.frame_by_can_protocol, &can_keys);
    log.frame_by_can_protocol.sort_by(|a, b| {
        let protocol_rank = |frame: &Frame| -> u8 { if frame.byte_length <= 8 { 0 } else { 1 } };

        match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let pr_ord: Ordering = protocol_rank(fa).cmp(&protocol_rank(fb));
                if pr_ord == Ordering::Equal {
                    let dlc_ord: Ordering = fa.byte_length.cmp(&fb.byte_length);
                    if dlc_ord == Ordering::Equal {
                        fallback_cmp(a, b)
                    } else {
                        dlc_ord
                    }
                } else {
                    pr_ord
                }
            }
            _ => fallback_cmp(a, b),
        }
    });

    refill(&mut log.frame_by_can_sender_node, &can_keys);
    log.frame_by_can_sender_node
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let node_a: &str = channel_map
                    .get(&fa.channel)
                    .and_then(|info| info.database.as_ref())
                    .and_then(|db| db.get_node_by_key(fa.tx_node_key))
                    .map(|node| node.name.as_str())
                    .unwrap_or("");
                let node_b: &str = channel_map
                    .get(&fb.channel)
                    .and_then(|info| info.database.as_ref())
                    .and_then(|db| db.get_node_by_key(fb.tx_node_key))
                    .map(|node| node.name.as_str())
                    .unwrap_or("");
                let node_ord: Ordering = node_a.cmp(node_b);
                if node_ord == Ordering::Equal {
                    fallback_cmp(a, b)
                } else {
                    node_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    refill(&mut log.frame_by_can_data, &can_keys);
    log.frame_by_can_data
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let data_ord: Ordering = fa.data.cmp(&fb.data);
                if data_ord == Ordering::Equal {
                    fallback_cmp(a, b)
                } else {
                    data_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    refill(&mut log.frame_by_can_comment, &can_keys);
    log.frame_by_can_comment
        .sort_by(|a, b| match (frames.get(*a), frames.get(*b)) {
            (Some(fa), Some(fb)) => {
                let comment_a: &str = channel_map
                    .get(&fa.channel)
                    .and_then(|info| info.database.as_ref())
                    .and_then(|db| db.get_message_by_key(fa.msg_key))
                    .map(|msg| msg.comment.as_str())
                    .unwrap_or("");
                let comment_b: &str = channel_map
                    .get(&fb.channel)
                    .and_then(|info| info.database.as_ref())
                    .and_then(|db| db.get_message_by_key(fb.msg_key))
                    .map(|msg| msg.comment.as_str())
                    .unwrap_or("");
                let comment_ord: Ordering = comment_a.cmp(comment_b);
                if comment_ord == Ordering::Equal {
                    fallback_cmp(a, b)
                } else {
                    comment_ord
                }
            }
            _ => fallback_cmp(a, b),
        });

    Ok(())
}
