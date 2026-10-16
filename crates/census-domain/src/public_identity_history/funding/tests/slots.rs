use super::*;

#[test]
fn full_slot_refusal_keeps_existing_values_and_charge() -> TestResult {
    let mut funding = TestFunding::new(16);
    let mut slots = OwnedHistorySlots::<u32, _>::with_capacity(2, &mut funding)?;
    slots.push(11)?;
    slots.push(22)?;
    check!(matches!(
        slots.push(33),
        Err(HistoryStorageError::Capacity {
            needed: 3,
            capacity: 2
        })
    ));
    check!(eq; slots.as_slice(), &[11, 22]);
    check!(eq; slots.pop(), Some(22));
    check!(eq; funding.pool().live(), 8);
    slots.push(44)?;
    check!(eq; slots.as_slice(), &[11, 44]);
    drop(slots);
    check!(eq; funding.pool().available(), 16);
    check!(funding.pool().valid());
    Ok(())
}

#[test]
fn empty_zero_sized_slots_succeed_but_nonempty_refuses_without_a_grant() -> TestResult {
    let mut funding = TestFunding::new(0);
    let empty = OwnedHistorySlots::<(), _>::with_capacity(0, &mut funding)?;
    check!(empty.is_empty());
    check!(eq; empty.capacity(), 0);
    check!(matches!(
        OwnedHistorySlots::<(), _>::with_capacity(1, &mut funding),
        Err(HistoryStorageError::ZeroSizedType)
    ));
    check!(eq; funding.calls(), 0);
    check!(eq; funding.pool().live(), 0);
    Ok(())
}

#[test]
fn overflowing_slot_layout_preserves_the_existing_owner() -> TestResult {
    let mut funding = TestFunding::new(16);
    let mut slots = OwnedHistorySlots::<u16, _>::with_capacity(2, &mut funding)?;
    slots.push(17)?;
    check!(matches!(
        OwnedHistorySlots::<u16, _>::with_capacity(usize::MAX, &mut funding),
        Err(HistoryStorageError::Overflow)
    ));
    check!(eq; funding.calls(), 1);
    check!(eq; slots.as_slice(), &[17]);
    check!(eq; funding.pool().live(), 4);
    drop(slots);
    check!(eq; funding.pool().available(), 16);
    Ok(())
}

struct Probe {
    pool: Rc<pool::Pool>,
    count: Rc<Cell<usize>>,
    valid: Rc<Cell<bool>>,
    held_bytes: u64,
}

impl Drop for Probe {
    fn drop(&mut self) {
        if self.pool.live() != self.held_bytes {
            self.valid.set(false);
        }
        let Some(count) = self.count.get().checked_add(1) else {
            self.valid.set(false);
            return;
        };
        self.count.set(count);
    }
}

#[test]
fn slot_payloads_are_destroyed_before_the_backing_grant_is_returned() -> TestResult {
    let bytes = std::mem::size_of::<Probe>()
        .checked_mul(2)
        .ok_or("probe layout overflow")?;
    let bytes = u64::try_from(bytes)?;
    let mut funding = TestFunding::new(bytes);
    let pool = Rc::clone(funding.pool());
    let count = Rc::new(Cell::new(0));
    let valid = Rc::new(Cell::new(true));
    let mut slots = OwnedHistorySlots::with_capacity(2, &mut funding)?;
    for _ in 0..2 {
        slots.push(Probe {
            pool: Rc::clone(&pool),
            count: Rc::clone(&count),
            valid: Rc::clone(&valid),
            held_bytes: bytes,
        })?;
    }
    drop(funding);
    check!(eq; pool.live(), bytes);
    drop(slots);
    check!(eq; count.get(), 2);
    check!(valid.get() && pool.valid());
    check!(eq; pool.live(), 0);
    check!(eq; pool.available(), bytes);
    Ok(())
}
