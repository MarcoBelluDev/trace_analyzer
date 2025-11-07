use chrono::NaiveDateTime;

/// Represents an absolute, timezone-unaware timestamp.
///
/// `AbsoluteTime` keeps both the raw textual representation (`text`) and the
/// parsed value as a `NaiveDateTime` (`value`). The timestamp is **naive**
/// (i.e., it has no timezone or offset information) and should not be used for
/// DST/offset-sensitive computations without attaching a timezone.
///
/// If the input line does not start with `"date"` or the timestamp does not
/// match the expected format, parsing returns `None`.
///
/// # Fields
/// - `text`: The raw timestamp string **after** the leading `"date "`
///   prefix (e.g., `"Tue Aug 05 07:23:45.123 pm 2025"`).
/// - `value`: The parsed timestamp as `Some(NaiveDateTime)` on success, or
///   `None` if not available.
///
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AbsoluteTime {
    pub text: String,
    pub value: Option<NaiveDateTime>,
}
impl AbsoluteTime {
    /// Clears all metadata from this `AbsoluteTime`.
    ///
    /// # Effects
    /// - `text` → `""`
    /// - `value` → `None`
    pub fn clear(&mut self) {
        self.text.clear();
        self.value = None;
    }
}
