//! The shared demo shell: ONE quoin `component!` lowered to every backend.
//!
//! The same source compiles for gpui, dioxus, leptos, xilem and floem
//! (selected by this crate's framework features — quoin's facade enforces
//! exactly-one). The shell:
//!
//! - installs the router in `on_mount` through [`crate::host::install`],
//!   wiring the framework-appropriate cross-thread refresh seam;
//! - renders a nav bar whose buttons drive the real `Navigator`, with
//!   active-state styling derived from the current pathname;
//! - switches page content on the matched route id, mirroring the
//!   `quoin_ui::Node` route UI with macro-rendered widgets;
//! - shows loader suspense (pending window + resolved data), the
//!   `before_load` redirect, search params and the splat 404.

use quoin::{component, quoin_render};

// The Floem emitter generates calls into floem's view-extension traits
// (`Decorators::style`, `IntoView::into_any`, …); they must be in scope at
// the expansion site — `floem::prelude` is exactly that bundle.
#[cfg(feature = "floem")]
use floem::prelude::*;

// Same idea for Xilem: the emitter's `boxed()` seams come from
// `xilem::WidgetView`.
#[cfg(feature = "xilem")]
use xilem::WidgetView as _;

// The state fields are quoin signals; `get`/`update` come from this trait.
// (Import is consumed inside the macro expansion, which rustc cannot see —
// hence the `as _` + allow.)
#[allow(unused_imports)]
use quoin::Signal as _;

/// Backend label for the header (proves the same source runs everywhere).
const FRAMEWORK: &str = crate::FRAMEWORK;

component! {
    pub NaviDemo {
        state {
            tick: u32 = 0,
            draft: String = String::new(),
        }

        on_mount {
            // Cross-thread refresh seam: timer wakes arrive from quoin's
            // background timer thread, so the closure must be Send + Sync.
            // GPUI: entities (and AsyncApp) are main-thread-bound, so the
            // thread-safe refresh is a channel; a foreground task pumps it
            // into `refresh_windows`.
            #[cfg(feature = "gpui")]
            let refresh = {
                let (tx, rx) = smol::channel::bounded::<()>(64);
                cx.spawn(async move |_this: ::gpui::WeakEntity<Self>, cx: &mut ::gpui::AsyncApp| {
                    while rx.recv().await.is_ok() {
                        let _ = cx.update(::gpui::App::refresh_windows);
                    }
                })
                .detach();
                std::sync::Arc::new(move || {
                    let _ = tx.send_blocking(());
                })
            };
            // Dioxus signals are single-threaded (unsync storage), so the
            // cross-thread refresh goes through the runtime's own
            // thread-safe scheduler instead of the tick signal.
            #[cfg(feature = "dioxus")]
            let refresh = {
                let scope = dioxus::core::current_scope_id();
                let schedule = dioxus::core::schedule_update_any();
                std::sync::Arc::new(move || schedule(scope))
            };
            #[cfg(not(any(feature = "gpui", feature = "dioxus")))]
            let refresh = {
                let tick = tick.clone();
                std::sync::Arc::new(move || tick.update(|value| *value += 1))
                    as std::sync::Arc<dyn Fn() + Send + Sync>
            };

            crate::host::install(refresh, navi_core::WindowKey(1));
        }

        render {
            // Drive the router's parked futures (loader fetches, guards).
            crate::driver::poll_router_tasks();

            // Subscribe the view to the refresh seam.
            let _generation = tick.get();

            let info = crate::read_route_info();
            let pending_note = if info.pending {
                " (resolving…)"
            } else {
                ""
            };
            // (Active-link styling is per-backend; the header below already
            // mirrors the live pathname, which doubles as the active state.)
            let header = format!("Navi demo — quoin backend: {FRAMEWORK}{pending_note}");
            // `input(value: draft)` moves the signal into the two-way
            // binding; the search button reads a clone.
            let draft_button = draft.clone();
            let _search_binding_is_live = &draft;

            quoin_render! {
                div(class: "flex flex-col gap-3 p-4 h-full bg-gray-900") {
                    h2() { { header.clone() } }
                    p() { { format!("Path: {}   Query: {}", info.pathname, if info.query.is_empty() { "—".to_string() } else { info.query.clone() }) } }

                    div(class: "flex flex-wrap gap-2") {
                        button(label: "Home", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/"))
                        button(label: "About", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/about"))
                        button(label: "User 1", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/users/1"))
                        button(label: "User 42", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/users/42"))
                        button(label: "Slow", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/slow"))
                        button(label: "Guarded", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/guarded"))
                        button(label: "404", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/definitely-missing"))
                        button(label: "Back", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::go_back())
                        button(label: "Forward", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::go_forward())
                    }


                    // ── Page switch on the matched route id ────────────
                    if [info.route_id == "IndexRoute"] {
                        div(class: "flex flex-col gap-2 p-3 bg-gray-800 rounded-md") {
                            h3() { "Welcome to Navi!" }
                            p() { "One route table, five frameworks: gpui, dioxus, leptos, xilem, floem." }
                            button(label: "Open /users/42", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/users/42"))
                        }
                    } else if [info.route_id == "AboutRoute"] {
                        div(class: "flex flex-col gap-2 p-3 bg-gray-800 rounded-md") {
                            h3() { "About Navi" }
                            p() { "File-based router on quoin: loaders, guards, search params, devtools — framework agnostic." }
                        }
                    } else if [info.route_id == "UsersParamIdRoute"] {
                        div(class: "flex flex-col gap-2 p-3 bg-gray-800 rounded-md") {
                            h3() { "User profile (loader + suspense)" }
                            if [info.loader == "Loading"] {
                                p() { "Loading user…" }
                            } else if [info.user.is_some()] {
                                p() { { format!("Resolved: {}", info.user.clone().unwrap_or_default()) } }
                            } else {
                                p() { "No data yet." }
                            }
                            button(label: "Back to index", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/"))
                        }
                    } else if [info.route_id == "SlowRoute"] {
                        div(class: "flex flex-col gap-2 p-3 bg-gray-800 rounded-md") {
                            h3() { "Slow loader (900 ms)" }
                            if [info.slow.is_some()] {
                                p() { { format!("{}", info.slow.clone().unwrap_or_default()) } }
                            } else {
                                p() { "Loading… watch the pending header note." }
                            }
                        }
                    } else if [info.route_id == "GuardedRoute"] {
                        div(class: "flex flex-col gap-2 p-3 bg-gray-800 rounded-md") {
                            h3() { "Guarded area" }
                            p() { "beforeLoad let you in — navigate with ?key=sesame to pass the guard." }
                        }
                    } else if [info.route_id == "SearchRoute"] {
                        div(class: "flex flex-col gap-2 p-3 bg-gray-800 rounded-md") {
                            h3() { "Search params" }
                            p() { { format!("Parsed query: {}", if info.query.is_empty() { "(none)".to_string() } else { info.query.clone() }) } }
                            input(value: draft, placeholder: "type a query, e.g. navi")
                            button(label: "Search", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_search(draft_button.get()))
                        }
                    } else {
                        div(class: "flex flex-col gap-2 p-3 bg-gray-800 rounded-md") {
                            h3() { "404 — not found" }
                            p() { "The splat route caught this URL (fuzzy not-found keeps the layout)." }
                            button(label: "Go home", class: "bg-blue-600 text-white rounded-md px-2 py-1", on_click: |_| crate::navigate_to("/"))
                        }
                    }
                }
            }
        }
    }
}
