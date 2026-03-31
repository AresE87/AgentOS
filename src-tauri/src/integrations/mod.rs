pub mod api_registry;
pub mod calendar;
pub mod database;
pub mod email;

pub use api_registry::{APIConnection, APIEndpoint, APIRegistry};
pub use calendar::{
    CalendarEvent, CalendarManager, CalendarProvider, GoogleCalendarProvider, TimeSlot,
};
pub use database::{ColumnInfo, DatabaseConfig, DatabaseManager, QueryResult, TableInfo};
pub use email::{EmailManager, EmailMessage, EmailTriage, GmailProvider, GOOGLE_COMBINED_SCOPES};
