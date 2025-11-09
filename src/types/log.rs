use dbc_editor::types::database::DatabaseDBC;
use slotmap::SlotMap;
use std::collections::HashMap;

use crate::types::absolute_time::AbsoluteTime;
use crate::types::frame::Frame;
use crate::types::keys::FrameKey;

#[derive(Clone, Debug, Default)]
pub struct Log {
    /// Vector containing all channelInfo starting from 0.
    pub channel_map: HashMap<u8, ChannelInfo>,

    /// Absolute start time extracted from the `date` header, if present.
    pub absolute_time: AbsoluteTime,

    /// All parsed frames in file order.
    pub frames: SlotMap<FrameKey, Frame>,

    // --- Order Complete "views"  ---
    pub frame_by_file_order: Vec<FrameKey>,
    pub frame_by_timestamp: Vec<FrameKey>,
    pub frame_by_channel: Vec<FrameKey>,
    pub frame_by_direction: Vec<FrameKey>,

    // --- Can order Complete "views"  ---
    pub frame_by_can_msg_name: Vec<FrameKey>,
    pub frame_by_can_msg_id: Vec<FrameKey>,
    pub frame_by_can_dlc: Vec<FrameKey>,
    pub frame_by_can_protocol: Vec<FrameKey>,
    pub frame_by_can_sender_node: Vec<FrameKey>,
    pub frame_by_can_data: Vec<FrameKey>,
    pub frame_by_can_comment: Vec<FrameKey>,
}

impl Log {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        *self = Log::default();
    }

    /// Check if there are any frames present
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn get_database_by_channel(&mut self, ch: u8) -> Option<&DatabaseDBC> {
        if let Some(ch_info) = self.channel_map.get(&ch) {
            ch_info.database.as_ref()
        } else {
            None
        }
    }

    pub fn get_mut_database_by_channel(&mut self, ch: u8) -> Option<&mut DatabaseDBC> {
        self.channel_map
            .get_mut(&ch)
            .and_then(|ch_info| ch_info.database.as_mut())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChannelInfo {
    pub number: u8,
    pub tipo: ChannelType,
    pub database: Option<DatabaseDBC>,
}
impl ChannelInfo {
    pub fn clear(&mut self) {
        *self = ChannelInfo::default();
    }
    pub fn db_name_to_string(&self) -> String {
        if let Some(db) = &self.database {
            db.name.clone()
        } else {
            "".to_string()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ChannelType {
    #[default]
    Can,
    Ethernet,
}
impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ChannelType::Can => "Can",
            ChannelType::Ethernet => "Ethernet",
        };
        f.write_str(label)
    }
}
