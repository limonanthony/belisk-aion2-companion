pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    bit_byte: u8,
    bits_left: u8,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            bit_byte: 0,
            bits_left: 0,
        }
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.offset == self.bytes.len()
    }

    pub(crate) fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(length)?;
        let taken = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(taken)
    }

    pub(crate) fn skip(&mut self, length: usize) -> Option<()> {
        self.take(length).map(|_| ())
    }

    pub(crate) fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|bytes| bytes[0])
    }

    pub(crate) fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }

    pub(crate) fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    pub(crate) fn i64(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    pub(crate) fn varint(&mut self, max_bytes: usize) -> Option<u64> {
        let mut value = 0u64;
        for index in 0..max_bytes {
            let byte = self.u8()?;
            let group = u64::from(byte & 0x7F);
            let shift = 7 * index as u32;
            if shift > 0 && (group << shift) >> shift != group {
                return None;
            }
            value |= group << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    pub(crate) fn varint32(&mut self) -> Option<u32> {
        u32::try_from(self.varint(5)?).ok()
    }

    pub(crate) fn varint64(&mut self) -> Option<u64> {
        self.varint(10)
    }

    pub(crate) fn entity_id(&mut self) -> Option<u32> {
        self.varint32().filter(|id| i32::try_from(*id).is_ok())
    }

    /// An element count that cannot exceed the bytes left, given each element
    /// takes at least `min_element_bytes`.
    pub(crate) fn count(&mut self, min_element_bytes: usize) -> Option<u32> {
        let count = self.varint32()?;
        let needed = usize::try_from(count).ok()?.checked_mul(min_element_bytes)?;
        (needed <= self.bytes.len() - self.offset).then_some(count)
    }

    pub(crate) fn skip_list(&mut self, element_bytes: usize) -> Option<()> {
        let count = self.count(element_bytes)?;
        self.skip(usize::try_from(count).ok()? * element_bytes)
    }

    /// A text with a varint byte length of at most `max_bytes`, without control
    /// characters.
    pub(crate) fn text(&mut self, max_bytes: usize) -> Option<String> {
        let length = usize::try_from(self.varint32()?).ok()?;
        if length > max_bytes {
            return None;
        }
        let text = String::from_utf8_lossy(self.take(length)?);
        Some(text.chars().filter(|character| !character.is_control()).collect())
    }

    /// Booleans share one byte, least significant bit first; a new byte is
    /// read at the position of the first boolean that needs it.
    pub(crate) fn bool_bit(&mut self) -> Option<bool> {
        if self.bits_left == 0 {
            self.bit_byte = self.u8()?;
            self.bits_left = 8;
        }
        let bit = (self.bit_byte >> (8 - self.bits_left)) & 1;
        self.bits_left -= 1;
        Some(bit == 1)
    }
}
