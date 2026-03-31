pub mod api_registry;
pub mod calendar;
pub mod database;
pub mod email;

pub use api_registry::{APIConnection, APIEndpoint, APIRegistry};
pub use calendar::{CalendarEvent, CalendarManager, CalendarProvider, GoogleCalendarProvider, TimeSlot};
pub use database::{DatabaseConfig, DatabaseManager, QueryResult, TableInfo, ColumnInfo};
pub use email::{EmailManager, EmailMessage, EmailTriage, GmailProvider, GOOGLE_COMBINED_SCOPES};
