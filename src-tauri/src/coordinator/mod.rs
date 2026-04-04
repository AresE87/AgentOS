pub mod event_bus;
pub mod planner;
pub mod pool;
pub mod remote_worker;
pub mod runtime;
pub mod scheduler;
pub mod specialists;
pub mod templates;
pub mod types;

pub use event_bus::{CoordinatorEvent, EventBus};
pub use planner::TaskPlanner;
pub use pool::AgentPool;
pub use runtime::CoordinatorRuntime;
pub use scheduler::TaskScheduler;
pub use specialists::{SpecialistCategory, SpecialistProfile, SpecialistRegistry};
pub use remote_worker::{RemoteWorkerHost, RemoteWorkerManager, RemoteWorkerResult};
pub use templates::MissionTemplates;
pub use types::{
    AgentAssignment, AgentLevel, AutonomyLevel, CoordinatorMode, DAGEdge, DAGNode, EdgeType,
    ExecutionTarget, Mission, MissionStatus, MissionSummary, NodePosition, SubtaskStatus, TaskDAG,
};
