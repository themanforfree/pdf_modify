use anyhow::Result;

use crate::config::SIG_CONTENTS_PLACEHOLDER_LEN;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ByteRange {
    pub(crate) byte_range_start: usize,
    pub(crate) byte_range_end: usize,
    pub(crate) value: [usize; 4],
}

impl ByteRange {
    pub(crate) fn from_bytes(buffer: &[u8]) -> Result<Self> {
        let byte_range_start = buffer
            .windows(b"/ByteRange[".len())
            .position(|window| window == b"/ByteRange[")
            .expect("ByteRange not found");
        let byte_range_end = buffer[byte_range_start..]
            .windows(b"]".len())
            .position(|window| window == b"]")
            .ok_or_else(|| anyhow::anyhow!("ByteRange end not found"))?
            + byte_range_start
            + 1;

        // only search prefix, position will return first match
        let contents_placeholder = hex::encode([b'0'; 50]).into_bytes();
        let contents_start = buffer
            .windows(contents_placeholder.len())
            .position(|w| w == contents_placeholder)
            .ok_or_else(|| anyhow::anyhow!("Cannot find Contents placeholder"))?;

        let value = [
            0,
            contents_start - 1,
            contents_start + SIG_CONTENTS_PLACEHOLDER_LEN * 2 + 1,
            buffer.len() - contents_start - SIG_CONTENTS_PLACEHOLDER_LEN * 2 - 1,
        ];
        Ok(ByteRange {
            byte_range_start,
            byte_range_end,
            value,
        })
    }

    pub(crate) fn get_bytes(&self) -> Vec<u8> {
        let mut bytes = format!(
            "/ByteRange[{} {} {} {}]",
            self.value[0], self.value[1], self.value[2], self.value[3]
        )
        .into_bytes();
        let pad_len = self.byte_range_end - self.byte_range_start - bytes.len();
        bytes.extend(b" ".repeat(pad_len));
        bytes
    }
}
