// navi-codegen/src/main.rs (new file)
use navi_codegen::{NaviConfig, generator::write_route_tree};

fn main() {
    let config = NaviConfig::from_file("navi.config.json").unwrap_or_else(|_| {
        eprintln!("No navi.config.json found, using defaults");
        NaviConfig::default()
    });
    write_route_tree(&config).expect("Failed to generate route tree");
    println!("Generated {}", config.generated_route_tree);
}
