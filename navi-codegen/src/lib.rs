pub mod scanner;
pub mod generator;
pub mod config;

pub use config::NaviConfig;
pub use scanner::scan_routes;
pub use generator::generate_route_tree;
