pub mod api_registry;
pub mod calendar;
pub mod database;
pub mod email;

pub use api_registry::{APIConnection, APIEndpoint, APIRegistry};
pub use calendar::{
    calendar_create_event, calendar_delete_event, calendar_list_calendars, calendar_list_events,
    calendar_update_event, CalendarEvent, CalendarManager, CalendarProvider,
    GoogleCalendarProvider, TimeSlot,
};
pub use database::{ColumnInfo, DatabaseConfig, DatabaseManager, QueryResult, TableInfo};
pub use email::{
    gmail_get_message, gmail_list_labels, gmail_list_messages, gmail_send, gmail_trash_message,
    refresh_google_token, EmailManager, EmailMessage, EmailTriage, GmailProvider,
    GOOGLE_COMBINED_SCOPES,
};
