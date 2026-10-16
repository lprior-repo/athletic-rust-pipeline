use super::ownership::Backing;
use super::{reserve_exact, HistoryFunding, HistoryGrant, HistoryStorageError};

pub struct OwnedHistoryBytes<G> {
    backing: Backing<Vec<u8>, G>,
}

impl<G: HistoryGrant> OwnedHistoryBytes<G> {
    pub fn with_capacity<F: HistoryFunding<Grant = G>>(
        capacity: usize,
        funding: &mut F,
    ) -> Result<Self, HistoryStorageError<F::Error>> {
        if capacity == 0 {
            return Ok(Self {
                backing: Backing::Empty,
            });
        }
        let grant = reserve_exact(capacity, funding)?;
        let mut value = Vec::new();
        value
            .try_reserve_exact(capacity)
            .map_err(HistoryStorageError::Allocation)?;
        if value.capacity() != capacity {
            return Err(HistoryStorageError::Capacity {
                needed: value.capacity(),
                capacity,
            });
        }
        Ok(Self {
            backing: Backing::Funded { value, grant },
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        match &self.backing {
            Backing::Empty => &[],
            Backing::Funded { value, .. } => value,
        }
    }

    pub fn capacity(&self) -> usize {
        match &self.backing {
            Backing::Empty => 0,
            Backing::Funded { value, .. } => value.capacity(),
        }
    }

    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    pub fn extend_from_slice(&mut self, bytes: &[u8]) -> Result<(), HistoryStorageError<G::Error>> {
        let needed = self
            .len()
            .checked_add(bytes.len())
            .ok_or(HistoryStorageError::Overflow)?;
        let capacity = self.capacity();
        if needed > capacity {
            return Err(HistoryStorageError::Capacity { needed, capacity });
        }
        if let Backing::Funded { value, .. } = &mut self.backing {
            value.extend_from_slice(bytes);
        }
        Ok(())
    }

    pub fn into_text(self) -> Result<OwnedHistoryText<G>, HistoryStorageError<G::Error>> {
        let backing = match self.backing {
            Backing::Empty => Backing::Empty,
            Backing::Funded { value, grant } => match String::from_utf8(value) {
                Ok(value) => Backing::Funded { value, grant },
                Err(error) => {
                    drop(error);
                    drop(grant);
                    return Err(HistoryStorageError::InvalidUtf8);
                }
            },
        };
        Ok(OwnedHistoryText { backing })
    }
}

pub struct OwnedHistoryText<G> {
    backing: Backing<String, G>,
}

impl<G: HistoryGrant> OwnedHistoryText<G> {
    pub fn as_text(&self) -> &str {
        match &self.backing {
            Backing::Empty => "",
            Backing::Funded { value, .. } => value,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_text().as_bytes()
    }

    pub fn capacity(&self) -> usize {
        match &self.backing {
            Backing::Empty => 0,
            Backing::Funded { value, .. } => value.capacity(),
        }
    }

    pub fn len(&self) -> usize {
        self.as_text().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_text().is_empty()
    }
}
