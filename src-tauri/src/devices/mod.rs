pub mod arvr;
pub mod car;
pub mod iot;
pub mod tablet;
pub mod tv;
pub mod wearable;

pub use arvr::{ARVRAgent, ARVRConfig};
pub use car::{CarAgent, CarConnection};
pub use iot::{IoTController, IoTDevice};
pub use tablet::{TabletConfig, TabletMode};
pub use tv::{TVConfig, TVDisplayMode};
pub use wearable::{WearableDevice, WearableManager};
