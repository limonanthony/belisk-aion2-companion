//! Cuts a reassembled server stream into packets and unwraps LZ4 bundles with
//! the same rules as `StreamProcessor::consume_stream` / `unwrap_bundle` in
//! meter-core, which also waits for a packet whose bytes have not all arrived,
//! however large.

use meter_core::capture::parser::utils::read_varint;
use lz4_flex::block::decompress;
use serde::Serialize;

const MAX_FRAME_LEN: usize = 65_535;
const MAX_BUNDLE_OUTPUT: usize = 5_000_000;
/// Frames above this size are counted in the stats.
const LARGE_FRAME_LEN: usize = 8 * 1024;
/// Capture-time seconds an incomplete head may wait before it is declared
/// garbage and skipped one byte.
const STALL_SECS: f64 = 5.0;

pub struct Frame {
    pub payload: Vec<u8>,
    pub in_bundle: bool,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStats {
    pub frames: u64,
    pub bundles_unwrapped: u64,
    pub bundles_failed: u64,
    pub skipped_bytes: u64,
    pub stall_skips: u64,
    pub largest_frame: usize,
    pub frames_over_8k: u64,
}

#[derive(Default)]
pub struct Framer {
    buffer: Vec<u8>,
    stalled_since: Option<f64>,
    pub stats: FrameStats,
}

impl Framer {
    pub fn leftover(&self) -> usize {
        self.buffer.len()
    }

    pub fn feed(&mut self, captured_at: f64, data: &[u8], out: &mut Vec<Frame>) {
        self.buffer.extend_from_slice(data);
        loop {
            let buffer = std::mem::take(&mut self.buffer);
            let consumed = self.scan(&buffer, out);
            self.buffer = buffer;
            self.buffer.drain(..consumed);
            if consumed > 0 {
                self.stalled_since = None;
            }
            if consumed > 0 && !self.buffer.is_empty() {
                continue;
            }
            if self.buffer.is_empty() {
                return;
            }
            match self.stalled_since {
                None => {
                    self.stalled_since = Some(captured_at);
                    return;
                }
                Some(since) if captured_at - since > STALL_SECS => {
                    self.buffer.drain(..1);
                    self.stats.stall_skips += 1;
                    self.stats.skipped_bytes += 1;
                    self.stalled_since = None;
                }
                Some(_) => return,
            }
        }
    }

    fn scan(&mut self, buffer: &[u8], out: &mut Vec<Frame>) -> usize {
        let mut offset = 0usize;
        while offset < buffer.len() {
            if buffer[offset] == 0 {
                offset += 1;
                continue;
            }
            let length = read_varint(buffer, offset);
            if !length.is_valid() || length.value <= 0 {
                if offset + 5 > buffer.len() {
                    break;
                }
                self.skip_byte(&mut offset);
                continue;
            }
            let total = match usize::try_from(length.value.saturating_sub(3)) {
                Ok(total) if total > 0 && total <= MAX_FRAME_LEN => total,
                _ => {
                    self.skip_byte(&mut offset);
                    continue;
                }
            };
            if offset + total > buffer.len() {
                break;
            }
            let packet = &buffer[offset..offset + total];
            let Some(payload_start) = payload_offset(packet, length.length) else {
                self.skip_byte(&mut offset);
                continue;
            };
            self.stats.largest_frame = self.stats.largest_frame.max(total);
            if total > LARGE_FRAME_LEN {
                self.stats.frames_over_8k += 1;
            }
            if is_bundle(packet, payload_start) {
                let bundle_end = offset + total + 1;
                if bundle_end > buffer.len() {
                    break;
                }
                self.unwrap_bundle(&buffer[offset + payload_start..bundle_end], out);
                offset = bundle_end;
            } else {
                self.emit(packet, false, out);
                offset += total;
            }
        }
        offset
    }

    fn skip_byte(&mut self, offset: &mut usize) {
        self.stats.skipped_bytes += 1;
        *offset += 1;
    }

    fn unwrap_bundle(&mut self, payload: &[u8], out: &mut Vec<Frame>) {
        if payload.len() < 7 {
            self.stats.bundles_failed += 1;
            return;
        }
        let size = u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]) as usize;
        if size == 0 || size > MAX_BUNDLE_OUTPUT {
            self.stats.bundles_failed += 1;
            return;
        }
        let Ok(inflated) = decompress(&payload[6..], size) else {
            self.stats.bundles_failed += 1;
            return;
        };
        self.stats.bundles_unwrapped += 1;

        let mut offset = 0usize;
        while offset < inflated.len() {
            if inflated[offset] == 0 {
                offset += 1;
                continue;
            }
            let length = read_varint(&inflated, offset);
            if !length.is_valid() || length.value <= 0 {
                break;
            }
            let Ok(total) = usize::try_from(length.value.saturating_sub(3)) else {
                break;
            };
            if total == 0 {
                offset += 1;
                continue;
            }
            let end = offset + total;
            if end > inflated.len() {
                break;
            }
            let inner = &inflated[offset..end];
            let Some(payload_start) = payload_offset(inner, length.length) else {
                break;
            };
            if is_bundle(inner, payload_start) {
                self.unwrap_bundle(&inner[payload_start..], out);
            } else {
                self.emit(inner, true, out);
            }
            offset = end;
        }
    }

    fn emit(&mut self, packet: &[u8], in_bundle: bool, out: &mut Vec<Frame>) {
        if packet.len() < 3 {
            return;
        }
        let length = read_varint(packet, 0);
        if !length.is_valid() {
            return;
        }
        let Some(payload_start) = payload_offset(packet, length.length) else {
            return;
        };
        let payload = &packet[payload_start..];
        if payload.len() < 2 {
            return;
        }
        self.stats.frames += 1;
        out.push(Frame { payload: payload.to_vec(), in_bundle });
    }
}

fn payload_offset(packet: &[u8], length_offset: usize) -> Option<usize> {
    let first = *packet.get(length_offset)?;
    let has_extension_byte = (0xF0..0xFF).contains(&first);
    let start = length_offset + usize::from(has_extension_byte);
    (start < packet.len()).then_some(start)
}

fn is_bundle(packet: &[u8], payload_start: usize) -> bool {
    packet.get(payload_start..payload_start + 2) == Some(&[0xFF, 0xFF])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(mut value: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let low = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                bytes.push(low);
                return bytes;
            }
            bytes.push(low | 0x80);
        }
    }

    fn length_header(bytes_after_header: usize) -> Vec<u8> {
        (1..=3usize)
            .map(|header_len| varint((header_len + bytes_after_header + 3) as u32))
            .zip(1..=3usize)
            .find(|(header, header_len)| header.len() == *header_len)
            .map(|(header, _)| header)
            .unwrap()
    }

    fn packet(opcode: [u8; 2], body_len: usize) -> Vec<u8> {
        let mut bytes = length_header(2 + body_len);
        bytes.extend_from_slice(&opcode);
        bytes.extend(std::iter::repeat(0xAB).take(body_len));
        bytes
    }

    #[test]
    fn a_packet_larger_than_the_product_threshold_survives_arriving_in_pieces() {
        let big = packet([0x0B, 0xE3], 13_000);
        let small = packet([0x01, 0x43], 20);
        let mut stream = big.clone();
        stream.extend_from_slice(&small);

        let mut framer = Framer::default();
        let mut out = Vec::new();
        for (index, piece) in stream.chunks(1460).enumerate() {
            framer.feed(index as f64 * 0.001, piece, &mut out);
        }

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].payload.len(), 13_002);
        assert_eq!(&out[1].payload[..2], &[0x01, 0x43]);
        assert_eq!(framer.stats.skipped_bytes, 0);
        assert_eq!(framer.stats.frames_over_8k, 1);
    }

    #[test]
    fn a_bundle_is_unwrapped_into_its_inner_packets() {
        let inner_a = packet([0x33, 0x36], 10);
        let inner_b = packet([0x0B, 0xE3], 30);
        let mut inflated = inner_a.clone();
        inflated.extend_from_slice(&inner_b);
        let compressed = lz4_flex::block::compress(&inflated);

        let mut payload = vec![0xFF, 0xFF];
        payload.extend_from_slice(&(inflated.len() as u32).to_le_bytes());
        payload.extend_from_slice(&compressed);
        let mut stream = length_header(payload.len() - 1);
        stream.extend_from_slice(&payload);

        let mut framer = Framer::default();
        let mut out = Vec::new();
        framer.feed(0.0, &stream, &mut out);

        let opcodes: Vec<_> = out.iter().map(|frame| [frame.payload[0], frame.payload[1]]).collect();
        assert_eq!(opcodes, vec![[0x33, 0x36], [0x0B, 0xE3]]);
        assert!(out.iter().all(|frame| frame.in_bundle));
        assert_eq!(framer.stats.bundles_unwrapped, 1);
    }
}
