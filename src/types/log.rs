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

    // --- All Frames Order by generic parameters  ---
    pub frame_by_file_order: Vec<FrameKey>,
    pub frame_by_timestamp: Vec<FrameKey>,
    pub frame_by_channel: Vec<FrameKey>,
    pub frame_by_direction: Vec<FrameKey>,

    // ---  All Frames Order by CAN parameters   ---
    pub frame_by_can_msg_name: Vec<FrameKey>,
    pub frame_by_can_msg_id: Vec<FrameKey>,
    pub frame_by_can_dlc: Vec<FrameKey>,
    pub frame_by_can_protocol: Vec<FrameKey>,
    pub frame_by_can_sender_node: Vec<FrameKey>,
    pub frame_by_can_data: Vec<FrameKey>,
    pub frame_by_can_comment: Vec<FrameKey>,

    // --- ID-Channel Order by generic parameters  ---
    pub id_chn_by_timestamp: Vec<FrameKey>,
    pub id_chn_by_channel: Vec<FrameKey>,
    pub id_chn_by_direction: Vec<FrameKey>,

    // --- ID-Channel Order by CAN parameters  ---
    pub id_chn_by_can_msg_name: Vec<FrameKey>,
    pub id_chn_by_can_msg_id: Vec<FrameKey>,
    pub id_chn_by_can_dlc: Vec<FrameKey>,
    pub id_chn_by_can_protocol: Vec<FrameKey>,
    pub id_chn_by_can_sender_node: Vec<FrameKey>,
    pub id_chn_by_can_data: Vec<FrameKey>,
    pub id_chn_by_can_comment: Vec<FrameKey>,
}

impl Log {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        *self = Log::default();
    }

    pub fn clear_frames(&mut self) {
        self.frames.clear();

        // --- All Frames Order by generic parameters  ---
        self.frame_by_file_order.clear();
        self.frame_by_timestamp.clear();
        self.frame_by_channel.clear();
        self.frame_by_direction.clear();

        // ---  All Frames Order by CAN parameters   ---
        self.frame_by_can_msg_name.clear();
        self.frame_by_can_msg_id.clear();
        self.frame_by_can_dlc.clear();
        self.frame_by_can_protocol.clear();
        self.frame_by_can_sender_node.clear();
        self.frame_by_can_data.clear();
        self.frame_by_can_comment.clear();

        // --- ID-Channel Order by generic parameters  ---
        self.id_chn_by_timestamp.clear();
        self.id_chn_by_channel.clear();
        self.id_chn_by_direction.clear();

        // --- ID-Channel Order by CAN parameters  ---
        self.id_chn_by_can_msg_name.clear();
        self.id_chn_by_can_msg_id.clear();
        self.id_chn_by_can_dlc.clear();
        self.id_chn_by_can_protocol.clear();
        self.id_chn_by_can_sender_node.clear();
        self.id_chn_by_can_data.clear();
        self.id_chn_by_can_comment.clear();
    }

    /// Check if there are any frames present
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn get_database_by_channel(&self, ch: u8) -> Option<&DatabaseDBC> {
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

    pub fn get_frame_by_key(&self, frame_key: &FrameKey) -> Option<&Frame> {
        self.frames.get(*frame_key)
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
