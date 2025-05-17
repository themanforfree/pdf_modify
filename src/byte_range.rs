use anyhow::Result;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ByteRange {
    pub(crate) byte_range_start: usize,
    pub(crate) byte_range_end: usize,
    pub(crate) value: [usize; 4],
}

impl ByteRange {
    pub(crate) fn from_bytes(buffer: &[u8]) -> Result<Self> {
        let byte_range_key = b"/ByteRange";
        let byte_range_start = buffer
            .windows(byte_range_key.len())
            .position(|window| window == byte_range_key)
            .expect("ByteRange not found");
        let byte_range_end = buffer[byte_range_start..]
            .windows(b"]".len())
            .position(|window| window == b"]")
            .ok_or_else(|| anyhow::anyhow!("ByteRange end not found"))?
            + byte_range_start
            + 1;

        // only search prefix, position will return first match
        let contents_placeholder = hex::encode([0; 50]).into_bytes();
        let contents_start = buffer
            .windows(contents_placeholder.len())
            .position(|w| w == contents_placeholder)
            .ok_or_else(|| anyhow::anyhow!("Cannot find Contents placeholder"))?
            - 1;
        let contents_end = buffer[contents_start..]
            .windows(b">".len())
            .position(|window| window == b">")
            .ok_or_else(|| anyhow::anyhow!("Contents end not found"))?
            + contents_start
            + 1;

        let value = [
            0,
            contents_start,
            contents_end,
            buffer.len() - contents_end - 1,
        ];
        Ok(ByteRange {
            byte_range_start,
            byte_range_end,
            value,
        })
    }

    pub(crate) fn get_bytes(&self) -> Vec<u8> {
        let mut bytes = format!(
            "/ByteRange [{} {} {} {}]",
            self.value[0], self.value[1], self.value[2], self.value[3]
        )
        .into_bytes();
        let pad_len = self.byte_range_end - self.byte_range_start - bytes.len();
        bytes.extend(b" ".repeat(pad_len));
        bytes
    }
}
