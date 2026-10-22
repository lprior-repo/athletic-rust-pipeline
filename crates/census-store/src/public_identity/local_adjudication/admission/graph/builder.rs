use super::*;

impl GraphPlanBuilder {
    pub(in crate::public_identity) fn new(capture: CaptureBindingV2) -> Self {
        Self {
            capture,
            descriptors: Vec::new(),
            scopes: Vec::new(),
            physical_rows: 0,
            metadata_bytes: 0,
        }
    }

    pub(in crate::public_identity) fn add(
        &mut self,
        store: &Store,
        node: bridge::Node<'_>,
    ) -> StoreResult<ObjectRefV2> {
        if matches!(&node, bridge::Node::Seal(_)) {
            return Err(invalid());
        }
        self.add_node(store, node)
    }

    fn add_node(&mut self, store: &Store, node: bridge::Node<'_>) -> StoreResult<ObjectRefV2> {
        let descriptors = self
            .descriptors
            .len()
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Self::require_plan_capacity("graph admission descriptors", descriptors, MAX_DESCRIPTORS)?;
        let scope = node.admission_table(&self.capture)?;
        let children = node.children()?;
        self.require_scope(scope, &children)?;
        validate::children_present(&self.descriptors, &children)?;
        let measured = node.measure(store)?;
        measured.audit(store)?;
        let object = ObjectRefV2::new(
            DigestV2::parse(measured.root())?,
            u32::try_from(measured.jsonl_bytes()).map_err(|_| StoreError::CounterOverflow)?,
            u32::try_from(measured.records()).map_err(|_| StoreError::CounterOverflow)?,
            measured.full_bytes(),
        )?;
        self.require_new_root(&object)?;
        let rows = self.require_measured_capacity(&measured)?;
        let mut references = Vec::new();
        references
            .try_reserve_exact(children.len())
            .map_err(|_| invalid())?;
        references.extend(children.into_iter().map(Reference));
        let descriptor = Descriptor {
            object: object.clone(),
            physical: measured.physical_ref().clone(),
            incremental: measured.incremental_charge().clone(),
            children: references,
        };
        let bytes = codec::encode(&descriptor, MAX_METADATA_BYTES)?.len();
        let metadata = self
            .metadata_bytes
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        Self::require_plan_capacity(
            "graph admission actual metadata bytes",
            metadata,
            MAX_METADATA_BYTES,
        )?;
        self.descriptors.try_reserve(1).map_err(|_| invalid())?;
        self.scopes.try_reserve(1).map_err(|_| invalid())?;
        self.descriptors.push(descriptor);
        self.scopes.push(scope);
        self.physical_rows = rows;
        self.metadata_bytes = metadata;
        Ok(object)
    }

    fn require_new_root(&self, object: &ObjectRefV2) -> StoreResult<()> {
        if self
            .descriptors
            .iter()
            .any(|entry| entry.object.root() == object.root())
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn require_scope(
        &self,
        scope: Option<crate::public_identity::paged_generation::AuthorityTableV2>,
        children: &[ObjectRefV2],
    ) -> StoreResult<()> {
        if let Some(table) = scope {
            for child in children {
                let index = self
                    .descriptors
                    .iter()
                    .position(|entry| &entry.object == child)
                    .ok_or_else(invalid)?;
                if self.scopes.get(index) != Some(&Some(table)) {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }

    pub(in crate::public_identity) fn finish(
        mut self,
        store: &Store,
        seal: &RawGenerationSealV2,
    ) -> StoreResult<GraphPlan> {
        if seal.binding() != &self.capture {
            return Err(invalid());
        }
        let object = self.add_node(store, bridge::Node::Seal(seal))?;
        validate::closure(&self.descriptors, &object)?;
        Ok(GraphPlan {
            capture: self.capture,
            seal: object,
            descriptors: self.descriptors,
        })
    }

    fn require_plan_capacity(
        what: &'static str,
        needed: usize,
        available: usize,
    ) -> StoreResult<()> {
        if needed > available {
            return Err(StoreError::BudgetExceeded {
                what,
                needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
                available: u64::try_from(available).map_err(|_| StoreError::CounterOverflow)?,
            });
        }
        Ok(())
    }

    fn require_measured_capacity(
        &self,
        measured: &crate::journal::context::ContextPhysicalPlan,
    ) -> StoreResult<usize> {
        let rows = self
            .physical_rows
            .checked_add(measured.row_count())
            .ok_or(StoreError::CounterOverflow)?;
        let projected = self
            .metadata_bytes
            .checked_add(measured.metadata_bytes_bound())
            .ok_or(StoreError::CounterOverflow)?;
        Self::require_plan_capacity(
            "graph admission cumulative physical rows",
            rows,
            MAX_PHYSICAL_ROWS,
        )?;
        Self::require_plan_capacity(
            "graph admission metadata bound bytes",
            projected,
            MAX_METADATA_BYTES,
        )?;
        Ok(rows)
    }
}
