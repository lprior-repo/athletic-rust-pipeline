#[derive(Debug)]
pub struct Discrepancy {
    pub message: String,
}

#[derive(Debug)]
pub struct EntityCheck {
    pub passed: usize,
    pub sampled_indices: Vec<usize>,
}
