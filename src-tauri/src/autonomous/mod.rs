pub mod inbox;
pub mod scheduling;
pub mod reporting;
pub mod data_entry;
pub mod qa;
pub mod support;
pub mod procurement;
pub mod compliance_auto;
pub mod reconciliation;

pub use inbox::{AutoInbox, InboxRule, AutoAction};
pub use scheduling::{AutoScheduler, SchedulingPreference, TimeBlock, TimeSlot, Suggestion};
pub use reporting::{AutoReporter, ReportConfig};
pub use data_entry::{AutoDataEntry, DataEntryTask, ValidationError};
pub use qa::{AutoQA, QACheck, CoverageReport};
pub use support::{AutoSupport, SupportTicket, SupportAction};
pub use procurement::{AutoProcurement, PurchaseRequest, SpendSummary};
pub use compliance_auto::{AutoCompliance, ComplianceTask};
pub use reconciliation::{AutoReconciliation, ReconciliationJob, Mismatch};
