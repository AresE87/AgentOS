mod bash;
mod calendar;
mod click;
mod edit_file;
mod email;
mod memory_search;
mod read_file;
mod screenshot;
mod search_files;
mod spawn_agent;
mod type_text;
mod web_browse;
mod web_search;
mod write_file;

pub use bash::BashTool;
pub use calendar::CalendarTool;
pub use click::ClickTool;
pub use edit_file::EditFileTool;
pub use email::EmailTool;
pub use memory_search::MemorySearchTool;
pub use read_file::ReadFileTool;
pub use screenshot::ScreenshotTool;
pub use search_files::SearchFilesTool;
pub use spawn_agent::SpawnAgentTool;
pub use type_text::TypeTextTool;
pub use web_browse::WebBrowseTool;
pub use web_search::WebSearchTool;
pub use write_file::WriteFileTool;

use super::ToolRegistry;

/// Register all 14 builtin tools into a registry
pub fn register_all(registry: &mut ToolRegistry) {
    registry.register(Box::new(BashTool));
    registry.register(Box::new(ReadFileTool));
    registry.register(Box::new(WriteFileTool));
    registry.register(Box::new(EditFileTool));
    registry.register(Box::new(SearchFilesTool));
    registry.register(Box::new(ScreenshotTool));
    registry.register(Box::new(ClickTool));
    registry.register(Box::new(TypeTextTool));
    registry.register(Box::new(WebBrowseTool));
    registry.register(Box::new(WebSearchTool));
    registry.register(Box::new(CalendarTool));
    registry.register(Box::new(EmailTool));
    registry.register(Box::new(MemorySearchTool));
    registry.register(Box::new(SpawnAgentTool));
}
