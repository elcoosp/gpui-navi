# Software Requirements Specification: Navi-Nexum Deep Link Integration (Optional Feature)

| Field | Value |
|-------|-------|
| Project | Navi Router — Nexum Deep Link Integration |
| Document | SRS (Feature Addendum) |
| Version | 0.2 (Draft) |
| Date | 2026-04-18 |
| Author | AI-assisted specification |
| Status | Draft — Pending Review |

## 1. Introduction and Scope

### 1.1 Purpose

This Software Requirements Specification defines the requirements for an **optional, deep integration** between the **Navi** router and the **Nexum** deep‑linking library. When enabled (via a Cargo feature flag), this integration provides:

- Automatic registration of custom URL schemes.
- Seamless navigation to the appropriate route when a deep link is received.
- A **dedicated "Deep Links" tab** within the Navi DevTools panel for inspecting incoming deep links, viewing history, and testing.

This feature is intended to be showcased in the `example-app` crate to demonstrate the combined capabilities of Navi and Nexum.

### 1.2 Scope

**In scope:**

- A new Cargo feature flag (e.g., `nexum`) in the `navi-router` and/or `navi-devtools` crates.
- A new integration module (e.g., `navi_router::deep_link` or a separate `navi-nexum` crate) that:
  - Configures Nexum with user‑provided URL schemes.
  - Spawns a background task to listen for deep links.
  - Converts received URLs to Navi `Location` objects.
  - Triggers navigation via `Navigator`.
  - (Optionally) logs events to the DevTools.
- A **dedicated DevTools tab** (when both `nexum` and `devtools` features are active) that displays:
  - A scrollable list of received deep link URLs with timestamps.
  - Status indicators (navigated successfully, ignored due to blocker, parse error).
  - Ability to clear history.
  - (Optional) an input field to simulate a deep link for testing.
- Updates to the `example-app` to demonstrate the integration with a custom scheme (e.g., `naviapp://`) and togglable devtools.

**Out of scope (Non‑goals):**

- Modifying Nexum’s core platform‑specific implementation.
- Changing Navi’s core routing or navigation logic.
- Supporting deep links when the application is **not running** (cold start) – Nexum already handles this, and the integration will receive the URL after the app activates.
- Providing UI for deep link management outside the DevTools.

### 1.3 Definitions, Acronyms, and Abbreviations

| Term | Definition |
|------|------------|
| **Deep Link** | A URL using a custom scheme (e.g., `naviapp://settings/profile`) that launches the app and passes the URL. |
| **Nexum** | A framework‑agnostic Rust crate for deep linking on Windows, macOS, and Linux. |
| **Navi** | A type‑safe router for GPUI applications. |
| **DevTools** | Navi’s built‑in developer tools panel (event timeline, cache inspector, etc.). |
| **Feature Flag** | A Cargo conditional compilation flag (`#[cfg(feature = "nexum")]`). |
| **Navigator** | Navi’s handle for programmatic navigation. |

### 1.4 References

- Nexum project (core + GPUI adapter)
- Navi project (router, macros, devtools)
- GPUI documentation

---

## 2. System Context and Overview

### 2.1 High‑Level Architecture

The integration sits between Nexum and Navi, with an optional extension into Navi DevTools.

```
┌─────────────┐     ┌─────────────────────────────┐     ┌─────────────┐
│   Nexum     │────▶│  Navi‑Nexum Integration     │────▶│    Navi     │
│ (URL events)│     │ (feature‑gated module)      │     │  (Router)   │
└─────────────┘     └─────────────┬───────────────┘     └─────────────┘
                                  │ (if devtools enabled)
                                  ▼
                        ┌─────────────────┐
                        │  Navi DevTools  │
                        │  (Deep Link Tab)│
                        └─────────────────┘
```

### 2.2 Key Capabilities

- **CAP‑INT‑1: Deep Link Reception & Navigation** – The application automatically navigates when a deep link is opened.
- **CAP‑INT‑2: DevTools Deep Link Tab** – Developers can inspect incoming deep links, see navigation results, and test deep links manually.
- **CAP‑INT‑3: Feature Flag Control** – The integration is entirely optional and excluded unless the `nexum` feature is enabled.

---

## 3. Functional Requirements

### 3.1 Feature Flag & Build Configuration

**FR‑FLAG‑001 – Cargo Feature `nexum`**  
The `navi-router` crate (or a new `navi-nexum` crate) shall provide a Cargo feature named `nexum`. When enabled, it shall:

- Add a dependency on `nexum-core` and `nexum-gpui`.
- Conditionally compile the integration module.

**FR‑FLAG‑002 – DevTools Feature Dependency**  
If both `nexum` and `devtools` features are enabled, the `navi-devtools` crate shall conditionally include the Deep Links tab UI and logic.

### 3.2 Integration Initialization

**FR‑INIT‑001 – Public Initialization API**  
The integration shall expose a public function, e.g., `navi_router::deep_link::init(config: Config, window_handle: AnyWindowHandle, cx: &mut App)`.  
This function shall:

- Call `nexum_gpui::setup_deep_links` with the provided `Config`.
- Obtain a `DeepLinkHandle`.
- Spawn a GPUI background task that listens for URLs.
- Store a handle to the background task (if needed for cleanup).

**FR‑INIT‑002 – Configuration**  
The `Config` type shall be re‑exported from `nexum_core` or wrapped for convenience, containing:

- `schemes: Vec<String>` – custom URL schemes to register.
- `app_links: Vec<AppLink>` – optional associated domains (macOS/iOS).

**FR‑INIT‑003 – Single Initialization**  
Calling the initialization function more than once shall either be a no‑op or return an error; the integration shall prevent multiple background listeners.

### 3.3 URL Reception and Navigation

**FR‑NAV‑001 – URL Parsing**  
Each received URL string shall be converted to a Navi `Location` using `Location::from_url`. If parsing fails, the error shall be logged and no navigation shall occur.

**FR‑NAV‑002 – Navigation**  
For a successfully parsed `Location`, the integration shall obtain a `Navigator` (using the stored `window_handle`) and call `navigator.push_location(loc, cx)`.

**FR‑NAV‑003 – Blocker Respect**  
Navigation triggered by a deep link shall respect any active Navi navigation blockers (automatically handled by `RouterState::navigate`).

**FR‑NAV‑004 – Error Logging**  
All errors (parse failures, navigation failures) shall be logged using the `log` crate at appropriate levels (`warn`, `error`).

### 3.4 DevTools Deep Links Tab

*The following requirements apply only when both `nexum` and `devtools` features are enabled.*

**FR‑TAB‑001 – Dedicated Tab**  
The Navi DevTools panel shall include a new tab labeled “Deep Links” (or similar). This tab shall be visible only when the `nexum` feature is active.

**FR‑TAB‑002 – Event History List**  
The tab shall display a scrollable list of received deep link events. Each entry shall show:

- **Timestamp** (local time, e.g., `HH:MM:SS`).
- **URL** (full string, truncated if necessary with tooltip).
- **Status** icon/text:
  - ✅ Navigated successfully
  - ⏸️ Blocked by navigation blocker
  - ❌ Parse error (invalid URL)
- Optional: **Matched Route ID** (if navigation succeeded).

**FR‑TAB‑003 – History Management**  
- The list shall retain the most recent `N` events (e.g., 100). Older events may be discarded.
- A “Clear History” button shall remove all events from the list.

**FR‑TAB‑004 – Manual Testing Input**  
The tab shall provide an input field and a “Simulate” button. When the user types a URL (e.g., `naviapp://test`) and clicks “Simulate”, the integration shall treat it exactly as if received from the OS (i.e., parse and navigate). This is invaluable for development testing.

**FR‑TAB‑005 – Integration with Event Bus**  
The background task shall emit events (e.g., `DeepLinkEvent { url, status, timestamp, matched_route }`) to a channel or global state that the DevTools tab subscribes to. This ensures the UI updates in real time.

### 3.5 Example App Showcase

**FR‑EXAMPLE‑001 – Feature Enabled**  
The `example-app` crate shall enable the `nexum` feature (and `devtools`) in its `Cargo.toml` to demonstrate the integration.

**FR‑EXAMPLE‑002 – URL Scheme Configuration**  
The example app shall register a custom scheme, e.g., `naviapp` (or `navi-example`).

**FR‑EXAMPLE‑003 – Demonstration Routes**  
The example app shall include at least one route that is reachable via deep link (e.g., `/settings`, `/users/42`). The root layout shall display a message like “Open `naviapp://settings` to test deep linking.”

**FR‑EXAMPLE‑004 – DevTools Visibility**  
The DevTools panel shall be visible by default (or togglable) and the Deep Links tab shall be populated when deep links are received.

---

## 4. Non‑Functional Requirements

### 4.1 Performance

**NFR‑PERF‑001 – URL Processing Overhead**  
The time from receiving a URL to dispatching the navigation (excluding Navi’s internal loader/blocker logic) shall be under 10 milliseconds.

**NFR‑PERF‑002 – Background Task Idle**  
The background listener shall not consume CPU when idle (blocking on `recv().await`).

### 4.2 Reliability

**NFR‑REL‑001 – Graceful Nexum Errors**  
If Nexum fails to register a scheme (e.g., missing permissions), the application shall continue to run and log an error; the deep link feature will simply not receive events.

**NFR‑REL‑002 – Thread Safety**  
All shared state (event history for DevTools) shall be protected by appropriate synchronization (`Mutex`, `RwLock`, or channels).

### 4.3 Usability (DevTools)

**NFR‑USA‑001 – Clear Feedback**  
The DevTools tab shall provide immediate visual feedback when a deep link is received (the list updates without requiring manual refresh).

**NFR‑USA‑002 – Intuitive Labels**  
Status icons shall have tooltips explaining their meaning.

### 4.4 Maintainability

**NFR‑MAINT‑001 – Isolation**  
Integration code shall be contained in a separate module (`navi_router::deep_link` or a dedicated crate) to minimize impact on core Navi when the feature is disabled.

**NFR‑MAINT‑002 – Documentation**  
Public APIs shall be documented with examples showing how to initialize the integration.

---

## 5. External Interfaces

### 5.1 Nexum APIs

The integration shall use:

- `nexum_core::Config`
- `nexum_gpui::setup_deep_links`
- `DeepLinkHandle::recv()`

### 5.2 Navi APIs

- `Location::from_url`
- `Navigator::new(window_handle)`
- `Navigator::push_location`
- (DevTools) `DevtoolsState` extension to register a custom tab provider.

### 5.3 DevTools Tab Registration

The `navi-devtools` crate shall expose a mechanism (e.g., a trait `DevtoolsTab`) that allows conditional registration of the Deep Links tab when the `nexum` feature is enabled.

---

## 6. Constraints, Assumptions, and Dependencies

### 6.1 Constraints

- **CON‑001 – GPUI Context:** Navigation must occur within a GPUI `App` context, so the background task must use `cx.update()`.
- **CON‑002 – Single Window:** The initial implementation assumes a single main window. Multi‑window support may be added later.

### 6.2 Assumptions

- **ASM‑001 – Navi Initialized First:** The integration initialization will be called after `RouterProvider` has been created and `RouterState` is set as a global.
- **ASM‑002 – Platform Support:** Nexum’s platform support (Windows, macOS, Linux) is sufficient; the integration does not need to add platform‑specific code.
- **ASM‑003 – DevTools Enabled:** The Deep Links tab is only available when the `devtools` feature is also enabled.

### 6.3 Dependencies

- `nexum-core` and `nexum-gpui` (when `nexum` feature is active).
- `log` crate for logging.
- `chrono` (optional, for timestamps in DevTools).

---

## 7. Requirements Attributes and Traceability

### 7.1 Priority (MoSCoW)

| ID | Priority | Rationale |
|----|----------|-----------|
| FR‑FLAG‑001 | Must | Core feature flag control. |
| FR‑INIT‑001 | Must | Essential initialization API. |
| FR‑NAV‑001 | Must | Core conversion and navigation. |
| FR‑TAB‑001 | Should | Deep Links tab greatly improves developer experience. |
| FR‑TAB‑004 | Could | Manual simulation is a nice‑to‑have for testing. |
| FR‑EXAMPLE‑001 | Should | Showcase is important for adoption. |

### 7.2 Traceability to Business Goals

| Business Goal | Requirement IDs |
|---------------|-----------------|
| Enable seamless deep linking in Navi‑based apps | FR‑FLAG‑001, FR‑INIT‑001, FR‑NAV‑001, FR‑NAV‑002 |
| Provide excellent developer experience for deep linking | FR‑TAB‑001, FR‑TAB‑002, FR‑TAB‑003, FR‑TAB‑004 |
| Demonstrate integration in example app | FR‑EXAMPLE‑001..004 |

---

## 8. Verification Approach (High‑Level)

| Requirement | Verification Method | Notes |
|-------------|---------------------|-------|
| FR‑FLAG‑001 | Inspection + Build | Verify that the feature flag correctly gates the code. |
| FR‑INIT‑001 | Unit Test + Manual | Test initialization with mock Nexum or on actual platform. |
| FR‑NAV‑001 | Unit Test | Test `Location::from_url` with sample URLs. |
| FR‑TAB‑002 | Manual Testing | Open DevTools, trigger deep links, verify list updates. |
| NFR‑PERF‑001 | Analysis | Code review; timing can be logged in debug builds. |
| NFR‑REL‑001 | Manual Test | Force scheme registration failure (e.g., no permissions) and ensure app continues. |

---

## 9. Appendices

### 9.1 Example Initialization Code (Illustrative)

```rust
// In main.rs after RouterProvider creation
#[cfg(feature = "nexum")]
{
    use navi_router::deep_link;
    let config = nexum_core::Config {
        schemes: vec!["naviapp".to_string()],
        app_links: vec![],
    };
    deep_link::init(config, window.window_handle(), cx);
}
```

### 9.2 DevTools Tab UI Mockup (Description)

```
┌─────────────────────────────────────────────────────────────┐
│  Timeline │ Cache │ Deep Links │ …                          │
├─────────────────────────────────────────────────────────────┤
│  [Simulate:] [naviapp://settings/profile] [Go] [Clear History] │
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ 14:32:05  ✅  naviapp://settings/profile  → /settings   ││
│  │ 14:31:22  ⏸️  naviapp://users/42  (blocked)             ││
│  │ 14:30:01  ❌  naviapp://invalid%%  (parse error)         ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### 9.3 TBD Log

| TBD ID | Description | Owner | Due Date |
|--------|-------------|-------|----------|
| TBD‑001 | Decide whether integration lives in `navi-router` or a separate `navi-nexum` crate. | Architect | Before implementation |
| TBD‑002 | Define exact `DevtoolsTab` trait and registration mechanism. | DevTools maintainer | Before implementation |
| TBD‑003 | Determine maximum history size (100? user‑configurable?). | UX/Dev | Before implementation |
