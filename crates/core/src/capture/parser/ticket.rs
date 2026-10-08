use crate::capture::parser::ticket_record::{decode_snapshot, decode_update};
use crate::capture::processor::StreamProcessor;

impl StreamProcessor {
    pub(crate) fn parse_ticket_snapshot_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(records) = decode_snapshot(packet) else {
            return false;
        };
        self.data_storage.tickets_snapshot(records, self.captured_at);
        true
    }

    pub(crate) fn parse_ticket_update_packet(&mut self, packet: &[u8], _is_compressed_bundle: bool) -> bool {
        let Ok(update) = decode_update(packet) else {
            return false;
        };
        self.data_storage
            .ticket_changed(update.record, update.reason, update.delta, self.captured_at);
        true
    }
}
