//! Criterion benchmarks for the navi-router adapter layer (per-frame costs).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use navi_router::components::link::path_is_active;
use navi_router::state::LocalFuture;
use navi_router::state::ScrollPositions;
use navi_router::{
    Location, NavigateOptions, RouteNode, RoutePattern, RouteTree, RouterHandle, RouterHost,
};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Queued host: spawned futures are parked (never run) — the benchmarks
/// exercise the synchronous state paths, which is what runs per frame.
#[derive(Default)]
struct BenchHost {
    queue: Mutex<Vec<LocalFuture>>,
    refresh_count: AtomicUsize,
}

impl BenchHost {
    fn host(self: &Rc<Self>) -> RouterHost {
        let queue = Rc::clone(self);
        let refresh = Rc::clone(self);
        RouterHost {
            refresh: Rc::new(move || {
                refresh.refresh_count.fetch_add(1, Ordering::SeqCst);
            }),
            spawn: Rc::new(move |future: LocalFuture| {
                queue.queue.lock().expect("queue").push(future);
            }),
        }
    }
}

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

fn setup() -> RouterHandle {
    let host = Rc::new(BenchHost::default());
    let mut tree = RouteTree::new();
    tree.add_route(node("home", "/"));
    tree.add_route(node("about", "/about"));
    tree.add_route(node("user", "/users/$id"));
    let handle = RouterHandle::new(
        Location::new("/"),
        navi_core::WindowKey(1),
        Arc::new(tree),
        host.host(),
    );
    handle.install_current();
    handle
}

fn bench_read_state(c: &mut Criterion) {
    let handle = setup();
    c.bench_function("state/current_location_clone", |b| {
        b.iter(|| handle.read().current_location());
    });
    c.bench_function("state/current_match_clone", |b| {
        b.iter(|| handle.read().current_match.clone());
    });
}

fn bench_scroll_positions(c: &mut Criterion) {
    // Pre-filled to capacity OUTSIDE the measured loop: each measured insert
    // evicts exactly one oldest entry (the O(n) -> O(1) eviction path).
    c.bench_function("scroll/insert_over_capacity", |b| {
        let sp = ScrollPositions::new();
        sp.set_capacity(128);
        #[expect(clippy::cast_precision_loss)]
        for i in 0..128 {
            sp.insert(format!("/route{i}"), i as f32);
        }
        let mut i = 0usize;
        b.iter(|| {
            let key = format!("/overflow{}", i % 16);
            i += 1;
            #[expect(clippy::cast_precision_loss)]
            let offset = i as f32;
            black_box(&sp).insert(key, offset);
        });
    });
    c.bench_function("scroll/get_hit", |b| {
        let sp = ScrollPositions::new();
        sp.insert("/route1".to_string(), 4.0);
        b.iter(|| sp.get(black_box("/route1")));
    });
}

fn bench_path_is_active(c: &mut Criterion) {
    c.bench_function("link/path_is_active_prefix", |b| {
        b.iter(|| path_is_active(black_box("/users/42/profile"), black_box("/users"), false));
    });
    c.bench_function("link/path_is_active_miss", |b| {
        b.iter(|| path_is_active(black_box("/users-admin"), black_box("/users"), false));
    });
}

fn bench_navigation(c: &mut Criterion) {
    // Replace navigations through the shared handle: one entry of history,
    // the full per-navigation adapter path (event dispatch, match, state).
    // The event log is cleared periodically to bound harness memory; the
    // push itself stays inside the measured path.
    c.bench_function("adapter/navigate_about", |b| {
        let handle = setup();
        let mut i = 0usize;
        b.iter(|| {
            handle.update(|state| {
                state.navigate(
                    Location::new("/about"),
                    &NavigateOptions {
                        replace: true,
                        ..NavigateOptions::default()
                    },
                );
            });
            i += 1;
            if i.is_multiple_of(1024) {
                handle.events().clear();
            }
        });
    });
}

criterion_group!(
    benches,
    bench_read_state,
    bench_scroll_positions,
    bench_path_is_active,
    bench_navigation
);
criterion_main!(benches);
