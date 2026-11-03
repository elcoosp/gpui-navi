//! Criterion benchmarks for the framework-agnostic router core.
//!
//! Groups mirror the two hot loops identified in `docs/PERFORMANCE.md`:
//! route matching (per navigation) and location parsing (per navigation).

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use navi_router_core::{Location, NavigateOptions, RouteNode, RoutePattern, RouteTree, RouterCore};
use std::collections::HashMap;
use std::sync::Arc;

/// Build the benchmark route tree: 130 routes mixing static, dynamic,
/// optional and splat segments (the shape matchit's own benchmarks use).
fn bench_tree() -> RouteTree {
    fn node(id: &str, pat: &str) -> RouteNode {
        RouteNode {
            id: id.into(),
            pattern: RoutePattern::parse(pat),
            parent: None,
            is_layout: false,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
            before_load: None,
            on_enter: None,
            on_leave: None,
            loader_deps: None,
            context_fn: None,
            meta: HashMap::new(),
        }
    }

    let mut tree = RouteTree::new();
    tree.add_route(node("root", "/"));
    for i in 0..30 {
        tree.add_route(node(&format!("s{i}"), &format!("/static/seg{i}")));
    }
    for i in 0..30 {
        tree.add_route(node(&format!("d{i}"), &format!("/users/u{i}/$id")));
    }
    for i in 0..20 {
        tree.add_route(node(
            &format!("o{i}"),
            &format!("/optional/o{i}/{{-$lang}}/docs"),
        ));
    }
    for i in 0..20 {
        tree.add_route(node(&format!("n{i}"), &format!("/nested/n{i}/a/b/$c/d")));
    }
    for i in 0..15 {
        tree.add_route(node(&format!("sp{i}"), &format!("/wild/w{i}/$")));
    }
    tree.add_route(node("mixed", "/users/new"));
    tree.add_route(node("deep", "/a/b/c/d/e/f/g/h/i/j/k"));
    tree.add_route(node("docs_splat", "/docs/$"));
    tree.add_route(node("catchall", "/$"));
    tree.add_route(node("root_layout", "/app"));
    tree
}

fn bench_match(c: &mut Criterion) {
    let tree = bench_tree();
    let mut group = c.benchmark_group("match_path");
    for (name, path) in [
        ("static", "/static/seg17"),
        ("dynamic", "/users/u12/9823"),
        ("static_beats_dynamic", "/users/new"),
        ("optional", "/optional/o4/en/docs"),
        ("deep_nested", "/nested/n7/a/b/xx/d"),
        ("splat", "/wild/w3/a/b/c"),
        ("root_catchall", "/totally/unknown/path"),
        ("deep_static", "/a/b/c/d/e/f/g/h/i/j/k"),
    ] {
        group.throughput(Throughput::Bytes(path.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), path, |b, p| {
            b.iter(|| tree.match_path(black_box(p)));
        });
    }
    group.finish();
}

fn bench_match_miss(c: &mut Criterion) {
    let tree = bench_tree();
    c.bench_function("match_path/no_match", |b| {
        b.iter(|| tree.match_path(black_box("/static/seg17/extra/deeper")));
    });
}

fn bench_location_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("location_new");
    for (name, path) in [
        ("plain", "/users/42"),
        ("query", "/search?q=navi&page=2&active=true"),
        ("query_hash", "/docs?page=1&tags=a&tags=b#section"),
        ("coerce", "/x?n=42&f=1.5&yes=true&no=false&empty=&s=abc"),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(name), path, |b, p| {
            b.iter(|| Location::new(black_box(p)));
        });
    }
    group.finish();
}

fn bench_history(c: &mut Criterion) {
    let tree = bench_tree();
    // `replace` navigations keep the history at one entry, so a single core
    // can serve millions of measured iterations without unbounded growth.
    c.bench_function("navigate/replace_static", |b| {
        let mut core = RouterCore::new(Location::new("/"), Arc::new(tree.clone()));
        b.iter(|| {
            core.navigate(
                Location::new("/static/seg3"),
                &NavigateOptions {
                    replace: true,
                    ..NavigateOptions::default()
                },
            )
        });
    });
    c.bench_function("navigate/replace_dynamic", |b| {
        let mut core = RouterCore::new(Location::new("/"), Arc::new(tree.clone()));
        b.iter(|| {
            core.navigate(
                Location::new("/users/u2/77"),
                &NavigateOptions {
                    replace: true,
                    ..NavigateOptions::default()
                },
            )
        });
    });
    c.bench_function("history/back_forward", |b| {
        let mut core = RouterCore::new(Location::new("/"), Arc::new(tree.clone()));
        core.navigate(Location::new("/static/seg3"), &NavigateOptions::default());
        b.iter(|| {
            let _ = core.back();
            let _ = core.forward();
        });
    });
    c.bench_function("transact/two_replaces", |b| {
        let mut core = RouterCore::new(Location::new("/"), Arc::new(tree.clone()));
        b.iter(|| {
            use navi_router_core::NavOp;
            core.transact(&[
                NavOp::Replace(Location::new("/static/seg3")),
                NavOp::Replace(Location::new("/users/u2/9")),
            ])
        });
    });
}

fn bench_ancestors(c: &mut Criterion) {
    // Chain of 4 layouts down to a leaf: __root layout chain depth 5.
    let mut tree = RouteTree::new();
    let mut parent: Option<String> = None;
    for (i, seg) in ["l0", "l1", "l2", "l3", "leaf"].iter().enumerate() {
        let pat = if *seg == "leaf" {
            "/seg3/leaf"
        } else {
            &format!("/seg{i}")
        };
        let n = RouteNode {
            id: (*seg).into(),
            pattern: RoutePattern::parse(pat),
            parent: parent.clone(),
            is_layout: i < 4,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
            before_load: None,
            on_enter: None,
            on_leave: None,
            loader_deps: None,
            context_fn: None,
            meta: HashMap::new(),
        };
        parent = Some((*seg).to_string());
        tree.add_route(n);
    }
    c.bench_function("tree/ancestors_depth5", |b| {
        b.iter(|| tree.ancestors(black_box("leaf")));
    });
}

criterion_group!(
    benches,
    bench_match,
    bench_match_miss,
    bench_location_new,
    bench_history,
    bench_ancestors
);
criterion_main!(benches);
