pub mod arvr;
pub mod wearable;
pub mod iot;
pub mod tablet;
pub mod tv;
pub mod car;

pub use arvr::{ARVRConfig, ARVRAgent};
pub use wearable::{WearableDevice, WearableManager};
pub use iot::{IoTDevice, IoTController};
pub use tablet::{TabletConfig, TabletMode};
pub use tv::{TVConfig, TVDisplayMode};
pub use car::{CarAgent, CarConnection};
