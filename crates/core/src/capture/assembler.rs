use std::sync::Arc;

use crate::capture::processor::StreamProcessor;
use crate::config::SharedDpsMeterConfig;
use crate::storage::data_storage::DataStorage;
use crate::logger::AppLogger;

const MAX_BUFFER_SIZE: usize = 50 * 1024 * 1024;
const INITIAL_CAPACITY: usize = 1024 * 1024;

pub struct StreamAssembler {
    buffer: Vec<u8>,
    buffer_limit: usize,
    processor: StreamProcessor,
}

impl StreamAssembler {
    pub fn new(
        data_storage: Arc<DataStorage>,
        logger: Arc<AppLogger>,
        port: String,
        config: SharedDpsMeterConfig,
    ) -> Self {
        Self {
            buffer: Vec::with_capacity(INITIAL_CAPACITY),
            buffer_limit: MAX_BUFFER_SIZE,
            processor: StreamProcessor::new(data_storage, logger, port, config),
        }
    }

    pub fn process_chunk(&mut self, data: &[u8], combat_damage_enabled: bool, captured_at: f64) -> bool {
        self.processor.set_captured_at(captured_at);
        self.processor
            .set_combat_damage_enabled(combat_damage_enabled);
        if self.buffer.len() + data.len() > self.buffer_limit {
            self.processor.logger.warn(format!(
                "[{}] stream buffer over {} bytes without a complete packet; discarding {} buffered bytes and re-framing",
                self.processor.port,
                self.buffer_limit,
                self.buffer.len()
            ));
            self.discard_buffered();
        }
        self.buffer.extend_from_slice(data);

        let mut consumed_total = 0;
        while consumed_total < self.buffer.len() {
            let consumed = self.processor.consume_stream(&self.buffer[consumed_total..]);
            if consumed == 0 {
                break;
            }
            consumed_total += consumed;
        }
        self.buffer.drain(..consumed_total);

        consumed_total > 0
    }

    pub fn clear(&mut self) {
        self.buffer = Vec::with_capacity(INITIAL_CAPACITY);
    }

    pub fn discard_buffered(&mut self) {
        self.buffer.clear();
        self.processor.reset_framing();
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    #[cfg(test)]
    pub(crate) fn with_buffer_limit(mut self, buffer_limit: usize) -> Self {
        self.buffer_limit = buffer_limit;
        self
    }
}
