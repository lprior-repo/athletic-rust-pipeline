use super::DepartureReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Work,
    Control,
    Drain,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotVector {
    pub tasks: u64,
    pub stages: u64,
    pub browser: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resources {
    pub bytes: u64,
    pub work: u64,
    pub slots: SlotVector,
}

impl Resources {
    pub(super) fn checked_add(self, other: Self) -> Result<Self, DepartureReason> {
        let add = |a: u64, b: u64| a.checked_add(b).ok_or(DepartureReason::AccountingOverflow);
        Ok(Self {
            bytes: add(self.bytes, other.bytes)?,
            work: add(self.work, other.work)?,
            slots: SlotVector {
                tasks: add(self.slots.tasks, other.slots.tasks)?,
                stages: add(self.slots.stages, other.slots.stages)?,
                browser: add(self.slots.browser, other.slots.browser)?,
            },
        })
    }

    pub(super) fn checked_sub(self, other: Self) -> Result<Self, DepartureReason> {
        let sub = |a: u64, b: u64| a.checked_sub(b).ok_or(DepartureReason::AccountingOverflow);
        Ok(Self {
            bytes: sub(self.bytes, other.bytes)?,
            work: sub(self.work, other.work)?,
            slots: SlotVector {
                tasks: sub(self.slots.tasks, other.slots.tasks)?,
                stages: sub(self.slots.stages, other.slots.stages)?,
                browser: sub(self.slots.browser, other.slots.browser)?,
            },
        })
    }

    pub(super) fn shortage(self, required: Self) -> Option<DepartureReason> {
        if required.bytes > self.bytes {
            return Some(DepartureReason::NoBytes);
        }
        if required.work > self.work {
            return Some(DepartureReason::NoWork);
        }
        if required.slots.tasks > self.slots.tasks
            || required.slots.stages > self.slots.stages
            || required.slots.browser > self.slots.browser
        {
            return Some(DepartureReason::NoSlots);
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkClass {
    pub resources: Resources,
    pub class: Class,
    pub origin: u64,
}

impl WorkClass {
    pub const fn work(bytes: u64, work: u64, slots: SlotVector) -> Self {
        Self {
            resources: Resources { bytes, work, slots },
            class: Class::Work,
            origin: 0,
        }
    }

    pub(super) fn validate(self) -> Result<(), DepartureReason> {
        let resources = self.resources;
        let empty = resources.bytes == 0
            && resources.work == 0
            && resources.slots.tasks == 0
            && resources.slots.stages == 0
            && resources.slots.browser == 0;
        if empty {
            return Err(DepartureReason::EmptyEnvelope);
        }
        Ok(())
    }
}

pub const WORK_CLASS_NORMAL: WorkClass = WorkClass::work(
    1024,
    1,
    SlotVector {
        tasks: 1,
        stages: 0,
        browser: 0,
    },
);
pub const WORK_CLASS_INGEST: WorkClass = WorkClass::work(
    0,
    0,
    SlotVector {
        tasks: 0,
        stages: 1,
        browser: 0,
    },
);
pub const WORK_CLASS_STAGE: WorkClass = WorkClass::work(
    256 * 1024 * 1024,
    100_000,
    SlotVector {
        tasks: 0,
        stages: 1,
        browser: 0,
    },
);
pub const WORK_CLASS_BROWSER: WorkClass = WorkClass::work(
    64 * 1024 * 1024,
    1,
    SlotVector {
        tasks: 0,
        stages: 0,
        browser: 1,
    },
);
pub const WORK_CLASS_CONTROL: WorkClass = WorkClass {
    resources: Resources {
        bytes: 64 * 1024,
        work: 1,
        slots: SlotVector {
            tasks: 0,
            stages: 1,
            browser: 0,
        },
    },
    class: Class::Control,
    origin: 0,
};
pub const WORK_CLASS_DRAIN: WorkClass = WorkClass {
    class: Class::Drain,
    ..WORK_CLASS_CONTROL
};
