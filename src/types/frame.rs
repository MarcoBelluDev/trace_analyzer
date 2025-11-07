use dbc_editor::types::database::{DatabaseDBC, MessageKey, NodeKey, SignalKey};

#[derive(Debug, Clone, Default)]
pub struct Frame {
    /// Absolute timestamp in `%Y-%m-%d %H:%M:%S%.3f` when available,
    /// otherwise derived by the parser.
    pub absolute_time: String,

    /// Relative timestamp in seconds since trace start (parsed from the textual token).
    pub timestamp: f64,

    /// Logger channel index (typically 1-based).
    pub channel: u8,

    /// FrameType
    pub ftype: FrameType,

    /// Direction as recorded by the logger, e.g. `"Rx"` or `"Tx"`.
    pub direction: Direction,

    /// ----- CAN Info ----- ///
    /// MessageKey from DatabaseDBC
    pub msg_key: MessageKey,
    /// Raw identifier as seen in the log
    pub id: u32,
    /// Raw identifier token as seen in the log
    pub id_hex: String,
    /// Raw payload length token as seen in the log
    pub byte_length: u16,
    /// First Sender NodeKey from DatabaseDBC
    pub tx_node_key: NodeKey,
    /// SignalKey from DatabaseDBC
    pub sig_keys: Vec<SignalKey>,

    /// Payload bytes as hex pairs separated by spaces.
    pub data: String,
}

impl Frame {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        *self = Frame::default();
    }

    /// Return the CAN Protocol of the Frame
    pub fn protocol_to_string(&self) -> String {
        if self.byte_length <= 8 {
            "CAN".to_string()
        } else {
            "CAN-FD".to_string()
        }
    }

    /// Return the CAN Msg Name of the Frame
    pub fn msg_name_to_string(&self, db: DatabaseDBC) -> String {
        if let Some(msg) = db.get_message_by_key(self.msg_key) {
            msg.name.clone()
        } else {
            "".to_string()
        }
    }

    /// Return the CAN Msg Name of the Frame
    pub fn msg_comment_to_string(&self, db: DatabaseDBC) -> String {
        if let Some(msg) = db.get_message_by_key(self.msg_key) {
            msg.comment.clone()
        } else {
            "".to_string()
        }
    }

    /// Return the CAN Msg Name of the Frame
    pub fn tx_node_name_to_string(&self, db: DatabaseDBC) -> String {
        if let Some(node) = db.get_node_by_key(self.tx_node_key) {
            node.name.clone()
        } else {
            "".to_string()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Direction {
    #[default]
    Rx,
    Tx,
}
impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label: &str = match self {
            Direction::Rx => "Rx",
            Direction::Tx => "Tx",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FrameType {
    #[default]
    Can,
    Eth,
    ErrorFrame,
}
impl std::fmt::Display for FrameType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label: &str = match self {
            FrameType::Can => "Can",
            FrameType::Eth => "Eth",
            FrameType::ErrorFrame => "ErrorFrame",
        };
        f.write_str(label)
    }
}
