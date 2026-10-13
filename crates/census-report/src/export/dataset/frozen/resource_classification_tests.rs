use super::codec::FrozenCodecError;
use super::codec_error;
use super::fixtures::TestResult;

#[test]
fn export_resource_limits_are_not_artifact_corruption() -> TestResult {
    let limits = [
        FrozenCodecError::RowTooLarge {
            what: "frozen line",
            bytes: 1_048_577,
            max: 1_048_576,
        },
        FrozenCodecError::BudgetExceeded {
            what: "frozen json depth",
            needed: 65,
            available: 64,
        },
        FrozenCodecError::NeedsPartition {
            what: "frozen inventory",
            bytes: 4_194_305,
            max: 4_194_304,
        },
    ];
    for limit in limits {
        check!(eq; codec_error(limit).corruption(), None);
    }
    Ok(())
}
