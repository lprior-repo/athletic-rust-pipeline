use super::session::{CaptureSession, Stage};
use crate::net::{CaptureTransportError, FetchError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::net) enum CaptureInterruption {
    None,
    BeforeArmTimeout,
    AfterArm,
    AfterObserved,
    AfterPrepared,
    AfterArchive,
}

impl CaptureInterruption {
    pub(super) fn at(self, point: Self) -> Result<(), FetchError> {
        if self == point {
            Err(CaptureTransportError::InjectedInterruption.into())
        } else {
            Ok(())
        }
    }
}

impl CaptureSession<'_> {
    pub(in crate::net) fn prearm_timeout(&self, url: &str) -> Result<(), FetchError> {
        if self.interruption() == CaptureInterruption::BeforeArmTimeout {
            return Err(FetchError::Timeout {
                url: url.to_owned(),
                timeout_secs: 15,
            });
        }
        Ok(())
    }

    pub(in crate::net) fn set_interruption(&mut self, value: CaptureInterruption) {
        if let Self::Adopted(adopted) = self {
            adopted.interruption = value;
        }
    }

    pub(in crate::net) fn interruption(&self) -> CaptureInterruption {
        match self {
            Self::Legacy => CaptureInterruption::None,
            Self::Adopted(adopted) => adopted.interruption,
        }
    }

    pub(in crate::net) fn interrupt(
        &mut self,
        point: CaptureInterruption,
    ) -> Result<(), FetchError> {
        if self.interruption() == point {
            if let Self::Adopted(adopted) = self {
                adopted.stage = Stage::Consumed;
            }
            return Err(CaptureTransportError::InjectedInterruption.into());
        }
        Ok(())
    }
}

#[test]
fn linear_capture_leases_keep_bounded_stack_representations(
) -> Result<(), Box<dyn std::error::Error>> {
    use super::session::{ArmedLease, DispatchLease};
    let word = std::mem::size_of::<usize>();
    check!(std::mem::size_of::<CaptureSession<'_>>() <= word * 2);
    check!(std::mem::size_of::<ArmedLease<'_>>() <= word * 3);
    check!(std::mem::size_of::<DispatchLease<'_>>() <= word * 3);
    Ok(())
}
