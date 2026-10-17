use std::sync::Arc;

use chromiumoxide::cdp::browser_protocol::network::{EventResponseReceived, RequestId};

use crate::transport::transfer::refuse_excess_wire;
use crate::BrowserError;

const MAX_PENDING: usize = 64;

struct PendingRow {
    id: RequestId,
    response: Option<Arc<EventResponseReceived>>,
    finished: Option<f64>,
}

#[derive(Default)]
pub(crate) struct PendingTransfers {
    rows: Vec<PendingRow>,
}

pub(crate) struct PendingTransfer {
    pub(crate) response: Option<Arc<EventResponseReceived>>,
    pub(crate) finished: Option<f64>,
}

impl PendingTransfers {
    fn find(&self, id: &RequestId) -> Option<usize> {
        self.rows.iter().position(|row| row.id == *id)
    }

    fn find_mut(&mut self, id: &RequestId) -> Option<&mut PendingRow> {
        self.rows.iter_mut().find(|row| row.id == *id)
    }

    pub(crate) fn observe_response(
        &mut self,
        event: Arc<EventResponseReceived>,
    ) -> Result<(), BrowserError> {
        if let Some(row) = self.find_mut(&event.request_id) {
            if row.response.is_some() {
                return Err(BrowserError::Protocol);
            }
            row.response = Some(event);
            return Ok(());
        }
        if self.rows.len() >= MAX_PENDING {
            return Err(BrowserError::PayloadLimit);
        }
        self.reserve_row()?;
        self.rows.push(PendingRow {
            id: event.request_id.clone(),
            response: Some(event),
            finished: None,
        });
        Ok(())
    }

    pub(crate) fn observe_finished(
        &mut self,
        id: &RequestId,
        encoded: f64,
    ) -> Result<(), BrowserError> {
        refuse_excess_wire(Some(encoded))?;
        if let Some(row) = self.find_mut(id) {
            if let Some(existing) = row.finished {
                if existing != encoded {
                    return Err(BrowserError::Protocol);
                }
                return Ok(());
            }
            row.finished = Some(encoded);
            return Ok(());
        }
        if self.rows.len() >= MAX_PENDING {
            return Err(BrowserError::PayloadLimit);
        }
        self.reserve_row()?;
        self.rows.push(PendingRow {
            id: id.clone(),
            response: None,
            finished: Some(encoded),
        });
        Ok(())
    }

    pub(crate) fn take(&mut self, id: &RequestId) -> Option<PendingTransfer> {
        let index = self.find(id)?;
        let row = self.rows.remove(index);
        Some(PendingTransfer {
            response: row.response,
            finished: row.finished,
        })
    }

    fn reserve_row(&mut self) -> Result<(), BrowserError> {
        if self.rows.capacity() == 0 {
            self.rows
                .try_reserve_exact(MAX_PENDING)
                .map_err(|_| BrowserError::Unavailable)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_json(id: &str, status: i64) -> serde_json::Value {
        serde_json::json!({
            "requestId": id,
            "loaderId": "loader-1",
            "timestamp": 1.0,
            "type": "Document",
            "response": {
                "url": "https://example.com/page",
                "status": status,
                "statusText": "response",
                "headers": {},
                "mimeType": "text/plain",
                "charset": "utf-8",
                "connectionReused": false,
                "connectionId": 1.0,
                "encodedDataLength": 128.0,
                "securityState": "unknown"
            },
            "hasExtraInfo": false
        })
    }

    fn make_response(id: &str, status: i64) -> Result<Arc<EventResponseReceived>, BrowserError> {
        let json = response_json(id, status);
        let event: EventResponseReceived =
            serde_json::from_value(json).map_err(|_| BrowserError::Protocol)?;
        Ok(Arc::new(event))
    }

    #[test]
    fn interleaved_response_and_finished_for_two_ids_preserve_exact_values(
    ) -> Result<(), BrowserError> {
        let mut ledger = PendingTransfers::default();
        let first_response = make_response("req-a", 200)?;
        let second_response = make_response("req-b", 403)?;

        ledger.observe_response(first_response)?;
        ledger.observe_finished(&RequestId::new("req-b"), 1024.0)?;
        ledger.observe_finished(&RequestId::new("req-a"), 2048.0)?;
        ledger.observe_response(second_response)?;

        let taken_a = ledger
            .take(&RequestId::new("req-a"))
            .ok_or(BrowserError::Protocol)?;
        assert!(taken_a.response.is_some());
        let resp_a = taken_a.response.as_ref().ok_or(BrowserError::Protocol)?;
        assert_eq!(resp_a.response.status, 200);
        assert_eq!(taken_a.finished, Some(2048.0));

        let taken_b = ledger
            .take(&RequestId::new("req-b"))
            .ok_or(BrowserError::Protocol)?;
        assert!(taken_b.response.is_some());
        let resp_b = taken_b.response.as_ref().ok_or(BrowserError::Protocol)?;
        assert_eq!(resp_b.response.status, 403);
        assert_eq!(taken_b.finished, Some(1024.0));

        Ok(())
    }

    #[test]
    fn sixty_fifth_distinct_id_refuses_without_losing_previous_sixty_four(
    ) -> Result<(), BrowserError> {
        let mut ledger = PendingTransfers::default();
        for index in 0..MAX_PENDING {
            let id = RequestId::new(index.to_string());
            ledger.observe_finished(&id, 100.0)?;
        }
        let excess_id = RequestId::new("excess");
        let result = ledger.observe_finished(&excess_id, 100.0);
        assert_eq!(result, Err(BrowserError::PayloadLimit));

        for index in 0..MAX_PENDING {
            let id = RequestId::new(index.to_string());
            let taken = ledger.take(&id).ok_or(BrowserError::Protocol)?;
            assert_eq!(taken.finished, Some(100.0));
        }
        Ok(())
    }

    #[test]
    fn duplicate_finished_with_different_total_refuses() -> Result<(), BrowserError> {
        let mut ledger = PendingTransfers::default();
        let id = RequestId::new("req-1");
        ledger.observe_finished(&id, 100.0)?;
        let result = ledger.observe_finished(&id, 200.0);
        assert_eq!(result, Err(BrowserError::Protocol));

        let taken = ledger.take(&id).ok_or(BrowserError::Protocol)?;
        assert_eq!(taken.finished, Some(100.0));
        Ok(())
    }

    #[test]
    fn duplicate_response_refuses() -> Result<(), BrowserError> {
        let mut ledger = PendingTransfers::default();
        let first = make_response("req-1", 200)?;
        ledger.observe_response(first)?;
        let second = make_response("req-1", 200)?;
        let result = ledger.observe_response(second);
        assert_eq!(result, Err(BrowserError::Protocol));
        Ok(())
    }

    #[test]
    fn malformed_numeric_total_is_refused_before_binding() -> Result<(), BrowserError> {
        let mut ledger = PendingTransfers::default();
        let id = RequestId::new("req-1");
        let result = ledger.observe_finished(&id, f64::NAN);
        assert_eq!(result, Err(BrowserError::Protocol));

        let result = ledger.observe_finished(&id, -1.0);
        assert_eq!(result, Err(BrowserError::Protocol));

        let result = ledger.observe_finished(&id, 0.5);
        assert_eq!(result, Err(BrowserError::Protocol));
        Ok(())
    }
}
