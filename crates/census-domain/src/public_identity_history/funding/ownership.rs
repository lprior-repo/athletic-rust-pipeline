pub(super) enum Backing<T, G> {
    Empty,
    Funded { value: T, grant: G },
}
