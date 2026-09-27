
mod canonical;
mod derived;
mod observations;

fn union_vec<T: PartialEq + Clone>(left: &mut Vec<T>, right: &[T]) {
    for item in right {
        if !left.contains(item) {
            left.push(item.clone());
        }
    }
}
