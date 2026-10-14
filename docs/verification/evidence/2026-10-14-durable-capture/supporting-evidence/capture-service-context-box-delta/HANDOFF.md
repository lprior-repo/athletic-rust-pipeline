# Private service capture-context boxing

TASK: address the reported 320-byte private enum variant without changing the capture ownership or public crawl context.

OWNERSHIP: detached delta only; Root integrates, formats and executes checks. No live edit, build or test was run.

OUTPUT: AdmittedCapture::DurableV1 now holds Box<ResultsCaptureContext>. Admission moves the constructed context into the box exactly once; the existing ResultsCaptureContext retains its non-cloneable RetryOwnerGrant and private mutex Available/Consumed state. context() borrows through Box::as_ref into the unchanged CaptureContext::DurableV1(&ResultsCaptureContext). Neither wrapper nor context gains Clone, and no optional/legacy fallback is introduced.

REGRESSION: admitted_capture_storage_stays_within_two_machine_words asserts the private wrapper fits within two machine words and that the explicit legacy branch still projects LegacyMetadata. It is proposed and unexecuted; existing integrated transport/replay tests remain the semantic evidence for linear grant consumption.

HANDOFF: context-box.patch modifies two exact-current files, with base/source trees and SHA-256 pins. Source inspection and contextual whitespace check only; no size measurement or passing regression is claimed.
