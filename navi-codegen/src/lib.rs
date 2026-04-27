pub mod config;
pub mod generator;
pub mod scanner;

pub use config::NaviConfig;
pub use generator::generate_route_tree;
pub use scanner::scan_routes;
