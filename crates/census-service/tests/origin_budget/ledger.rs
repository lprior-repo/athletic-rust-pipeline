use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{CLOCK_TOLERANCE_MS, DELAY_MS, INFLIGHT_BUDGET, OWNER_AGENT, TARGET};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Request {
    pub connection: usize,
    pub start_ns: u64,
    pub end_ns: u64,
    pub method: String,
    pub path: String,
    pub user_agent: String,
    pub response_status: u16,
}

#[derive(Debug, Serialize)]
pub(super) enum Event {
    Start {
        connection: usize,
        at_ns: u64,
        method: String,
        path: String,
        user_agent: String,
    },
    End {
        connection: usize,
        at_ns: u64,
        response_status: u16,
    },
}

#[derive(Debug, Serialize)]
pub(super) struct Measurements {
    pub max_inflight: usize,
    pub min_spacing_ns: u64,
    pub max_burst: usize,
    pub burst_window_ns: u64,
    pub admission_intervals: usize,
    pub admissions_per_second: f64,
}

pub(super) fn assemble(events: Vec<Event>) -> Result<Vec<Request>> {
    let mut requests = BTreeMap::new();
    for event in events {
        match event {
            Event::Start {
                connection,
                at_ns,
                method,
                path,
                user_agent,
            } => {
                let record = Request {
                    connection,
                    start_ns: at_ns,
                    end_ns: 0,
                    method,
                    path,
                    user_agent,
                    response_status: 0,
                };
                ensure!(
                    requests.insert(connection, record).is_none(),
                    "duplicate physical start"
                );
            }
            Event::End {
                connection,
                at_ns,
                response_status,
            } => {
                let record = requests
                    .get_mut(&connection)
                    .ok_or_else(|| anyhow::anyhow!("end without start"))?;
                ensure!(record.response_status == 0, "duplicate physical end");
                record.end_ns = at_ns;
                record.response_status = response_status;
            }
        }
    }
    Ok(requests.into_values().collect())
}

pub(super) fn qualify(requests: &[Request]) -> Result<Measurements> {
    ensure!(
        requests.len() == 2,
        "unexpected physical traffic: {requests:?}"
    );
    let paths: Vec<_> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    ensure!(
        paths == ["/robots.txt", TARGET],
        "wrong exact physical ledger: {requests:?}"
    );
    for request in requests {
        ensure!(
            request.method == "GET",
            "unexpected physical method: {request:?}"
        );
        ensure!(
            request.user_agent == OWNER_AGENT,
            "rival emitted physical traffic: {request:?}"
        );
        ensure!(
            request.response_status == 200 && request.end_ns >= request.start_ns,
            "incomplete HTTP response: {request:?}"
        );
    }
    let first = requests
        .first()
        .ok_or_else(|| anyhow::anyhow!("no robots request"))?;
    let last = requests
        .last()
        .ok_or_else(|| anyhow::anyhow!("no target request"))?;
    let spacing = last
        .start_ns
        .checked_sub(first.start_ns)
        .ok_or_else(|| anyhow::anyhow!("nonmonotonic admissions"))?;
    let window = DELAY_MS
        .checked_sub(CLOCK_TOLERANCE_MS)
        .and_then(|millis| millis.checked_mul(1_000_000))
        .ok_or_else(|| anyhow::anyhow!("invalid clock tolerance"))?;
    ensure!(
        spacing >= window,
        "physical starts spaced {spacing}ns, required >= {window}ns (1000ms delay, 50ms tolerance)"
    );
    let peak = peak_inflight(requests)?;
    let burst = peak_burst(requests, window);
    ensure!(
        peak <= INFLIGHT_BUDGET,
        "physical in-flight budget multiplied: {peak}"
    );
    ensure!(burst == 1, "physical burst budget multiplied: {burst}");
    Ok(Measurements {
        max_inflight: peak,
        min_spacing_ns: spacing,
        max_burst: burst,
        burst_window_ns: window,
        admission_intervals: 1,
        admissions_per_second: 1.0 / std::time::Duration::from_nanos(spacing).as_secs_f64(),
    })
}

pub(super) fn peak_inflight(requests: &[Request]) -> Result<usize> {
    let mut boundaries = Vec::with_capacity(requests.len().saturating_mul(2));
    for request in requests {
        boundaries.push((request.start_ns, 1_i8));
        boundaries.push((request.end_ns, -1_i8));
    }
    boundaries.sort_unstable();
    let mut inflight = 0_usize;
    let mut peak = 0;
    for (_, delta) in boundaries {
        inflight = if delta == 1 {
            inflight.checked_add(1)
        } else {
            inflight.checked_sub(1)
        }
        .ok_or_else(|| anyhow::anyhow!("invalid physical in-flight accounting"))?;
        peak = peak.max(inflight);
    }
    ensure!(inflight == 0, "physical requests not drained");
    Ok(peak)
}

pub(super) fn peak_burst(requests: &[Request], window: u64) -> usize {
    requests
        .iter()
        .map(|start| {
            requests
                .iter()
                .filter(|candidate| {
                    candidate.start_ns >= start.start_ns
                        && candidate.start_ns.saturating_sub(start.start_ns) < window
                })
                .count()
        })
        .max()
        .map_or(0, |peak| peak)
}
