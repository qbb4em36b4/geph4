use std::{collections::BTreeSet, time::Instant};

use bytes::Bytes;

use crate::mux::structs::*;

use super::inflight::Inflight;

pub(crate) struct ConnVars {
    pub inflight: Inflight,
    pub next_free_seqno: Seqno,
    pub retrans_count: u64,

    pub delayed_ack_timer: Option<Instant>,
    pub ack_seqnos: BTreeSet<Seqno>,

    pub reorderer: Reorderer<Bytes>,
    pub lowest_unseen: Seqno,
    // read_buffer: VecDeque<Bytes>,
    slow_start: bool,
    pub cwnd: f64,
    last_loss: Instant,

    flights: u64,
    last_flight: Instant,

    loss_rate: f64,

    pub closing: bool,
}

impl Default for ConnVars {
    fn default() -> Self {
        ConnVars {
            inflight: Inflight::new(),
            next_free_seqno: 0,
            retrans_count: 0,

            delayed_ack_timer: None,
            ack_seqnos: BTreeSet::new(),

            reorderer: Reorderer::default(),
            lowest_unseen: 0,

            slow_start: true,
            cwnd: 16.0,
            last_loss: Instant::now(),

            flights: 0,
            last_flight: Instant::now(),

            loss_rate: 0.0,

            closing: false,
        }
    }
}

impl ConnVars {
    fn cwnd_target(&self) -> f64 {
        (self.inflight.bdp() * 2.0).min(10000.0).max(16.0)
    }

    pub fn pacing_rate(&self) -> f64 {
        self.inflight.rate() * 2.0
    }

    pub fn congestion_ack(&mut self) {
        self.loss_rate *= 0.99;
        let n = 0.23 * self.cwnd.powf(0.4).max(1.0);
        self.cwnd += n / self.cwnd;
        let now = Instant::now();
        if now.saturating_duration_since(self.last_flight) > self.inflight.srtt() {
            self.flights += 1;
            self.last_flight = now
        }
    }

    pub fn congestion_loss(&mut self) {
        self.slow_start = false;
        self.loss_rate = self.loss_rate * 0.99 + 0.01;
        let now = Instant::now();
        if now.saturating_duration_since(self.last_loss) > self.inflight.srtt() {
            self.cwnd *= 0.8;
            log::debug!(
                "LOSS CWND => {}; loss rate {}, srtt {}ms",
                self.cwnd,
                self.loss_rate,
                self.inflight.srtt().as_millis()
            );
            self.last_loss = now;
        }
    }
}
