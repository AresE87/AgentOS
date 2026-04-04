pub mod traits;
pub mod twitter;
pub mod linkedin;
pub mod reddit;
pub mod hackernews;
pub mod manager;

pub use traits::{SocialPlatform, PostResult, Mention, EngagementMetrics};
pub use manager::SocialManager;
