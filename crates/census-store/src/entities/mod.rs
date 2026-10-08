mod canonical;
mod derived;
mod observations;

fn union_vec<T: PartialEq>(left: &mut Vec<T>, right: Vec<T>) {
    right.into_iter().for_each(|item| {
        if !left.contains(&item) {
            left.push(item);
        }
    });
}
