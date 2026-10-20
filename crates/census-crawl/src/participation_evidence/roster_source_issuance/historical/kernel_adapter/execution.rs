use super::{EXECUTING_CONFIGURATION, PORTABLE_EXECUTING_CONFIGURATION};

pub(super) const RAW_VALUE_NATIVE_CONFIGURATION: [u8; 32] = [
    0xa0, 0x14, 0x53, 0xe2, 0x39, 0x14, 0x25, 0xf2, 0xcb, 0x50, 0x26, 0x00, 0x28, 0xd1, 0xf5, 0xd7,
    0x5a, 0xd2, 0x3e, 0x5a, 0xde, 0xf7, 0xfb, 0xa8, 0x07, 0xec, 0x95, 0xdb, 0x38, 0xd3, 0x78, 0x7d,
];

pub(super) fn compatible(executing: &[u8; 32], required: &[u8; 32]) -> bool {
    if executing == required {
        return true;
    }
    if executing == &PORTABLE_EXECUTING_CONFIGURATION && required == &EXECUTING_CONFIGURATION {
        return true;
    }
    if executing == &RAW_VALUE_NATIVE_CONFIGURATION && required == &EXECUTING_CONFIGURATION {
        return true;
    }
    false
}
