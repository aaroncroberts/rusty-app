# Component-Based Docking Architecture

**Status**: Design Complete (Implementation Pending)
**Version**: 1.0
**Last Updated**: 2026-02-17

## Overview

This document describes the component-based docking architecture for rusty-app, a flexible system where UI components (Servers, Tables, Properties, Editor) can be registered as "Views" and docked in application regions (LeftPanel, MainPanel). Views can be toggled via the View menu with checkmarks indicating enabled state. Only enabled views appear as tabs in their docked region.

### Goals

1. **Dynamic UI Composition**: Components can be enabled/disabled at runtime
2. **Type-Safe Abstraction**: Enum-based dispatch eliminates string-based lookups
3. **Extensible Design**: Adding new components or regions requires minimal changes
4. **Separation of Concerns**: Clear boundaries between data state and UI state
5. **Iced-Friendly**: Leverages Iced's Elm-inspired architecture

## Architecture Layers

The architecture consists of four layers, each with a distinct responsibility:

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Region (LeftPanelRegion, MainPanelRegion)         │
│ - Dockable areas that host components                      │
│ - Manage active tab state (UI state)                       │
│ - Query ViewRegistry for enabled views                     │
│ - Render tabs and delegate to active component             │
└─────────────────────────────────────────────────────────────┘
                            ↓ queries
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: ViewRegistry                                       │
│ - Centralized registry (single source of truth)            │
│ - HashMap<ComponentId, View> for O(1) operations           │
│ - Enable/disable/toggle operations                         │
│ - Query enabled views per region                           │
└─────────────────────────────────────────────────────────────┘
                            ↓ owns
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: View                                               │
│ - Wraps Component with enabled state                       │
│ - Associates component with RegionId                       │
│ - Default enabled: true                                    │
└─────────────────────────────────────────────────────────────┘
                            ↓ wraps
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Component trait (ServerList, TableList, etc.)     │
│ - Stateless UI components                                  │
│ - Implement view() method for rendering                    │
│ - Emit ComponentAction via callback                        │
└─────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Purpose | State | Example |
|-------|---------|-------|---------|
| **Component** | Render UI | None (stateless) | `ServerListComponent.view()` renders server list |
| **View** | Add visibility toggle | `enabled: bool`, `region: RegionId` | View wraps ServerList, enabled=true, region=LeftPanel |
| **ViewRegistry** | Manage enabled state | `HashMap<ComponentId, View>` | Registry tracks which views are enabled |
| **Region** | Layout & composition | `active_tab: Option<ComponentId>` | LeftPanelRegion queries registry, renders tabs |

## Core Types

### 1. ComponentId Enum

Type-safe identifier for each component.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    ServerList,
    TableList,
    Properties,
    Editor,
    ConnectionForm,
}

impl ComponentId {
    pub fn all() -> &'static [ComponentId] {
        &[
            ComponentId::ServerList,
            ComponentId::TableList,
            ComponentId::Properties,
            ComponentId::Editor,
            ComponentId::ConnectionForm,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ComponentId::ServerList => "Servers",
            ComponentId::TableList => "Tables",
            ComponentId::Properties => "Properties",
            ComponentId::Editor => "Query Editor",
            ComponentId::ConnectionForm => "Connection Manager",
        }
    }
}
```

**Design Rationale:**
- **Enum over strings**: Compile-time safety, no typos, exhaustive matching
- **Copy + Hash**: Efficient HashMap keys, no allocations
- **Centralized names**: UI consistency guaranteed

### 2. ComponentAction Enum

Unified action type with nested component-specific actions.

```rust
#[derive(Debug, Clone)]
pub enum ComponentAction {
    ServerList(ServerListAction),
    TableList(TableListAction),
    Properties(PropertiesAction),
    Editor(EditorAction),
    ConnectionForm(ConnectionFormAction),
}

// Example component-specific actions
#[derive(Debug, Clone)]
pub enum ServerListAction {
    SelectServer(String),
    NewConnection,
    RefreshList,
}

#[derive(Debug, Clone)]
pub enum EditorAction {
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    QueryChanged(String),
    ExecuteQuery,
}
```

**Design Rationale:**
- **Nested enums**: Each component has isolated action space, no naming conflicts
- **Clone**: Required for Iced message handling
- **Extensible**: Adding new components adds one variant

### 3. Component Trait

Stateless UI components with generic view method.

```rust
use iced::Element;
use crate::theme::ThemeColors;

pub trait Component: Send {
    /// Unique identifier for this component
    fn id(&self) -> ComponentId;

    /// Display name for tabs and menus
    fn title(&self) -> &str {
        self.id().name()
    }

    /// Render the component with generic message mapping
    fn view<'a, Message: 'a + Clone>(
        &'a self,
        theme: ThemeColors,
        on_action: impl Fn(ComponentAction) -> Message + 'a,
    ) -> Element<'a, Message>;
}
```

**Design Rationale:**
- **Stateless**: State lives in DatabaseIDE, not in components
- **Send bound**: Required for `Box<dyn Component>` trait objects
- **Generic view()**: Allows composition into any Message type
- **Callback pattern**: Component emits actions, caller maps to messages

### 4. RegionId Enum

Type-safe region identification.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionId {
    LeftPanel,
    MainPanel,
}

impl RegionId {
    pub fn all() -> &'static [RegionId] {
        &[RegionId::LeftPanel, RegionId::MainPanel]
    }

    pub fn name(&self) -> &'static str {
        match self {
            RegionId::LeftPanel => "Left Panel",
            RegionId::MainPanel => "Main Panel",
        }
    }
}
```

**Design Rationale:**
- **Enum for type safety**: No string-based region lookups
- **Copy + Hash**: Efficient for HashMap keys
- **Extensible**: Future regions (RightPanel, BottomPanel) just add variants

### 5. View Struct

Component wrapper with visibility state.

```rust
pub struct View {
    component: Box<dyn Component>,
    enabled: bool,
    region: RegionId,
}

impl View {
    pub fn new(component: Box<dyn Component>, region: RegionId) -> Self {
        Self {
            component,
            enabled: true,  // Default to enabled
            region,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn region(&self) -> RegionId {
        self.region
    }

    pub fn component(&self) -> &dyn Component {
        self.component.as_ref()
    }
}
```

**Design Rationale:**
- **Wraps Box<dyn Component>**: Dynamic dispatch for trait objects
- **enabled: bool**: Toggle-able visibility state
- **region: RegionId**: Fixed region assignment per view
- **Default enabled: true**: Components visible by default

### 6. ViewRegistry

Centralized view management with O(1) operations.

```rust
use std::collections::HashMap;

pub struct ViewRegistry {
    views: HashMap<ComponentId, View>,
}

impl ViewRegistry {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    pub fn register(&mut self, component: Box<dyn Component>, region: RegionId) {
        let id = component.id();
        self.views.insert(id, View::new(component, region));
    }

    pub fn enable(&mut self, id: ComponentId) -> bool {
        if let Some(view) = self.views.get_mut(&id) {
            view.enable();
            true
        } else {
            false
        }
    }

    pub fn disable(&mut self, id: ComponentId) -> bool {
        if let Some(view) = self.views.get_mut(&id) {
            view.disable();
            true
        } else {
            false
        }
    }

    pub fn toggle(&mut self, id: ComponentId) -> bool {
        if let Some(view) = self.views.get_mut(&id) {
            view.toggle();
            true
        } else {
            false
        }
    }

    pub fn is_enabled(&self, id: ComponentId) -> bool {
        self.views
            .get(&id)
            .map(|view| view.is_enabled())
            .unwrap_or(false)
    }

    pub fn enabled_views(&self, region: RegionId) -> Vec<&View> {
        self.views
            .values()
            .filter(|view| view.region() == region && view.is_enabled())
            .collect()
    }

    pub fn region_views(&self, region: RegionId) -> Vec<&View> {
        self.views
            .values()
            .filter(|view| view.region() == region)
            .collect()
    }
}
```

**Design Rationale:**
- **HashMap over Vec**: O(1) lookups by ComponentId (vs O(n) linear search)
- **Owns Views**: Single source of truth for component state
- **Returns Vec<&View>**: Simple API, collection small (~10 views)
- **enabled_views()**: Filters by region AND enabled state

### 7. Region Trait

Polymorphic interface for dockable areas.

```rust
pub trait Region {
    fn id(&self) -> RegionId;

    fn active_component(&self) -> Option<ComponentId>;

    fn set_active_component(&mut self, component_id: ComponentId);

    fn view<'a, Message: 'a + Clone>(
        &'a self,
        registry: &'a ViewRegistry,
        theme: ThemeColors,
        on_action: impl Fn(ComponentAction) -> Message + 'a,
    ) -> Element<'a, Message>;
}
```

**Design Rationale:**
- **Trait over enum**: Different regions have different state (width, visibility)
- **active_component**: UI state (which tab is selected)
- **view() takes registry**: Render-time queries ensure dynamic updates
- **Generic Message**: Allows composition into DatabaseIDE's message type

### 8. LeftPanelRegion

Tabbed region with resize and visibility.

```rust
pub struct LeftPanelRegion {
    active_tab: Option<ComponentId>,
    width: f32,
    visible: bool,
}

impl LeftPanelRegion {
    pub fn new() -> Self {
        Self {
            active_tab: None,
            width: 250.0,
            visible: true,
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width.max(100.0).min(600.0);
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}

impl Region for LeftPanelRegion {
    fn id(&self) -> RegionId {
        RegionId::LeftPanel
    }

    fn active_component(&self) -> Option<ComponentId> {
        self.active_tab
    }

    fn set_active_component(&mut self, component_id: ComponentId) {
        self.active_tab = Some(component_id);
    }

    fn view<'a, Message: 'a + Clone>(
        &'a self,
        registry: &'a ViewRegistry,
        theme: ThemeColors,
        on_action: impl Fn(ComponentAction) -> Message + 'a,
    ) -> Element<'a, Message> {
        if !self.visible {
            return container(row![]).into();
        }

        let enabled_views = registry.enabled_views(RegionId::LeftPanel);

        // Render tabs from enabled views
        let tabs = render_tabs(enabled_views, self.active_tab, theme, |id| {
            Message::SelectTab(RegionId::LeftPanel, id)
        });

        // Render active component content
        let content = if let Some(active_id) = self.active_tab {
            if let Some(view) = enabled_views.iter().find(|v| v.component().id() == active_id) {
                view.component().view(theme, on_action)
            } else {
                container(text("No component selected")).into()
            }
        } else {
            container(text("No component selected")).into()
        };

        container(column![tabs, content].spacing(0))
            .width(self.width)
            .into()
    }
}
```

**Design Rationale:**
- **width: f32**: Resize state for drag-to-resize
- **visible: bool**: Toggle-able via View menu
- **Query enabled_views()**: Dynamic tabs based on registry state
- **Delegate rendering**: Calls `view.component().view()`

### 9. MainPanelRegion

Tabbed region that fills available space.

```rust
pub struct MainPanelRegion {
    active_tab: Option<ComponentId>,
}

impl MainPanelRegion {
    pub fn new() -> Self {
        Self {
            active_tab: None,
        }
    }
}

impl Region for MainPanelRegion {
    fn id(&self) -> RegionId {
        RegionId::MainPanel
    }

    fn active_component(&self) -> Option<ComponentId> {
        self.active_tab
    }

    fn set_active_component(&mut self, component_id: ComponentId) {
        self.active_tab = Some(component_id);
    }

    fn view<'a, Message: 'a + Clone>(
        &'a self,
        registry: &'a ViewRegistry,
        theme: ThemeColors,
        on_action: impl Fn(ComponentAction) -> Message + 'a,
    ) -> Element<'a, Message> {
        let enabled_views = registry.enabled_views(RegionId::MainPanel);

        let tabs = render_tabs(enabled_views, self.active_tab, theme, |id| {
            Message::SelectTab(RegionId::MainPanel, id)
        });

        let content = if let Some(active_id) = self.active_tab {
            if let Some(view) = enabled_views.iter().find(|v| v.component().id() == active_id) {
                view.component().view(theme, on_action)
            } else {
                container(text("No component selected")).into()
            }
        } else {
            container(text("No component selected")).into()
        };

        container(column![tabs, content].spacing(0)).into()
    }
}
```

**Design Rationale:**
- **No width management**: Fills available space
- **Always visible**: No visibility toggle (main content area)
- **Same tab pattern**: Reuses render_tabs helper

## Design Decisions & Rationale

### 1. Stateless Components

**Decision**: Components do not store state; state lives in DatabaseIDE.

**Rationale:**
- Iced's architecture: `update()` handles state, `view()` renders
- Components are trait objects (`Box<dyn Component>`)
- Storing state in trait objects complicates lifetimes
- Centralized state easier to serialize/debug
- Matches Iced's "single state tree" pattern

**Implication**: Components receive all needed data via `view()` parameters

### 2. HashMap in ViewRegistry

**Decision**: Use `HashMap<ComponentId, View>` instead of `Vec<View>`.

**Rationale:**
- O(1) lookup by ComponentId for enable/disable operations
- No linear search needed for queries
- ComponentId is Copy + Hash (perfect for HashMap key)
- Natural mapping: one view per component ID

**Trade-off:** Slightly more memory overhead, but typically <10 views

### 3. Enum Dispatch Pattern

**Decision**: Use enums for ComponentId, ComponentAction, RegionId instead of strings.

**Rationale:**
- Compile-time safety (no typos)
- Exhaustive matching in match statements
- HashMap key with no allocations (Copy)
- Easier refactoring (find all references)
- Better IDE support

**Alternative Rejected**: String-based IDs (`&'static str`) - runtime errors, no exhaustive matching

### 4. Fixed Region Per View

**Decision**: Each View has a fixed RegionId (not dynamically re-assignable).

**Rationale:**
- Components are designed for specific regions
- ServerList naturally belongs in LeftPanel
- Editor naturally belongs in MainPanel
- Simplifies queries (no need to track region separately)

**Future Extension**: If drag-and-drop docking is added, could make region mutable via `set_region()`

### 5. Active Tab in Region (Not ViewRegistry)

**Decision**: Regions store `active_tab: Option<ComponentId>`, not ViewRegistry.

**Rationale:**
- Active tab is UI state (which tab is selected)
- Enabled state is data state (which components are available)
- Separation of concerns: ViewRegistry manages availability, Region manages selection
- Multiple regions can have different active tabs

**Example:**
```rust
// ViewRegistry: "Editor is enabled in MainPanel"
registry.is_enabled(ComponentId::Editor)  // true

// MainPanelRegion: "Editor tab is currently active"
main_panel.active_tab  // Some(ComponentId::Editor)
```

### 6. Render-Time Queries

**Decision**: Regions query ViewRegistry at render time, not during initialization.

**Rationale:**
- Enabled views can change dynamically (View menu toggles)
- Render-time queries ensure tabs always reflect current state
- Single source of truth (ViewRegistry)
- No synchronization issues

**Performance**: Query is fast (HashMap filter, O(n) where n = total components, typically <10)

### 7. Generic view() Method

**Decision**: `view<Message>()` instead of `view() -> Element<RegionMessage>`.

**Rationale:**
- Allows `Box<dyn Component>` (trait object)
- Same component can render in different message contexts
- Callback pattern familiar from existing code
- More flexible for composition

**Alternative Rejected**: Associated type `type Message` - cannot use trait objects

### 8. Region Trait (Not Enum)

**Decision**: Use `trait Region` instead of `enum RegionType`.

**Rationale:**
- Each region has different state (LeftPanel has width, MainPanel doesn't)
- Extensible: adding new region types doesn't require modifying core enum
- Allows region-specific methods (e.g., `set_width()` only on LeftPanel)
- Matches Rust best practices for polymorphism

**Alternative Considered**: `enum RegionType { LeftPanel { width, visible, ... }, MainPanel { ... } }` - less clean API

## Integration Patterns

### DatabaseIDE State

```rust
pub struct DatabaseIDE {
    // Existing fields
    theme: ThemeColors,

    // View registry (centralized data state)
    view_registry: ViewRegistry,

    // Regions (UI state)
    left_panel: LeftPanelRegion,
    main_panel: MainPanelRegion,
}
```

**Key Points:**
- ViewRegistry: Single source of truth for enabled state
- Regions: Concrete structs (not `Box<dyn Region>`)
- Each region manages its own active tab

### Message Handling

```rust
pub enum Message {
    // Existing variants
    MenuAction(MenuAction),
    ComponentAction(ComponentAction),
    SelectTab(RegionId, ComponentId),
    TogglePanel(RegionId),
    ResizePanel(RegionId, f32),
}

impl DatabaseIDE {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MenuAction(MenuAction::View(view_item)) => {
                if let Some(component_id) = view_item.as_component_id() {
                    self.view_registry.toggle(component_id);

                    // Auto-switch if toggling off active view
                    if self.left_panel.active_component() == Some(component_id) {
                        let enabled = self.view_registry.enabled_views(RegionId::LeftPanel);
                        if let Some(first) = enabled.first() {
                            self.left_panel.set_active_component(first.component().id());
                        } else {
                            self.left_panel.set_active_component(None);
                        }
                    }
                }
                Task::none()
            }
            Message::SelectTab(region_id, component_id) => {
                match region_id {
                    RegionId::LeftPanel => {
                        self.left_panel.set_active_component(component_id);
                    }
                    RegionId::MainPanel => {
                        self.main_panel.set_active_component(component_id);
                    }
                }
                Task::none()
            }
            Message::TogglePanel(RegionId::LeftPanel) => {
                self.left_panel.toggle_visibility();
                Task::none()
            }
            Message::ComponentAction(action) => {
                // Route to appropriate handler based on action variant
                match action {
                    ComponentAction::ServerList(server_action) => {
                        // Handle server list actions
                    }
                    ComponentAction::Editor(editor_action) => {
                        // Handle editor actions
                    }
                    // ... other components
                }
                Task::none()
            }
            // ... other messages
        }
    }
}
```

### Initialization

```rust
impl DatabaseIDE {
    pub fn new() -> Self {
        let mut view_registry = ViewRegistry::new();

        // Register components with regions
        view_registry.register(
            Box::new(ServerListComponent),
            RegionId::LeftPanel
        );
        view_registry.register(
            Box::new(TableListComponent),
            RegionId::LeftPanel
        );
        view_registry.register(
            Box::new(PropertiesComponent),
            RegionId::LeftPanel
        );
        view_registry.register(
            Box::new(EditorComponent),
            RegionId::MainPanel
        );
        view_registry.register(
            Box::new(ConnectionFormComponent),
            RegionId::MainPanel
        );

        let mut left_panel = LeftPanelRegion::new();
        let mut main_panel = MainPanelRegion::new();

        // Initialize active tabs from enabled views
        if let Some(first) = view_registry.enabled_views(RegionId::LeftPanel).first() {
            left_panel.set_active_component(first.component().id());
        }
        if let Some(first) = view_registry.enabled_views(RegionId::MainPanel).first() {
            main_panel.set_active_component(first.component().id());
        }

        Self {
            view_registry,
            left_panel,
            main_panel,
            // ... other fields
        }
    }
}
```

### Rendering

```rust
impl DatabaseIDE {
    fn view(&self) -> Element<Message> {
        let menu_bar = MenuBar::new().view(
            &self.theme,
            |action| Message::MenuAction(action)
        );

        let left_panel_view = self.left_panel.view(
            &self.view_registry,
            self.theme,
            |action| Message::ComponentAction(action)
        );

        let main_panel_view = self.main_panel.view(
            &self.view_registry,
            self.theme,
            |action| Message::ComponentAction(action)
        );

        container(
            column![
                menu_bar,
                row![
                    left_panel_view,
                    main_panel_view
                ]
            ]
        )
        .into()
    }
}
```

### View Menu Integration

```rust
pub enum ViewMenuItem {
    ToggleLeftPanel,
    Servers,
    Tables,
    Properties,
    QueryEditor,
    ConnectionManager,
}

impl ViewMenuItem {
    pub fn as_component_id(&self) -> Option<ComponentId> {
        match self {
            ViewMenuItem::ToggleLeftPanel => None,
            ViewMenuItem::Servers => Some(ComponentId::ServerList),
            ViewMenuItem::Tables => Some(ComponentId::TableList),
            ViewMenuItem::Properties => Some(ComponentId::Properties),
            ViewMenuItem::QueryEditor => Some(ComponentId::Editor),
            ViewMenuItem::ConnectionManager => Some(ComponentId::ConnectionForm),
        }
    }
}

// In MenuBar::view() - View menu items
let view_menu = ViewMenuItem::all()
    .iter()
    .map(|item| {
        if let Some(component_id) = item.as_component_id() {
            let is_enabled = registry.is_enabled(component_id);

            button(
                row![
                    if is_enabled { "✓ " } else { "  " },
                    text(item.name())
                ]
            )
            .on_press(Message::MenuAction(MenuAction::View(*item)))
        } else {
            // ToggleLeftPanel - no checkmark
            button(text(item.name()))
                .on_press(Message::MenuAction(MenuAction::View(*item)))
        }
    });
```

## Lifecycle & Data Flow

### Startup

```
1. DatabaseIDE::new()
   ├─ Create ViewRegistry
   ├─ Register all components with regions
   │  ├─ ServerList → LeftPanel (enabled by default)
   │  ├─ TableList → LeftPanel (enabled by default)
   │  ├─ Properties → LeftPanel (enabled by default)
   │  ├─ Editor → MainPanel (enabled by default)
   │  └─ ConnectionForm → MainPanel (enabled by default)
   ├─ Create LeftPanelRegion
   ├─ Create MainPanelRegion
   └─ Initialize active tabs from enabled views
```

### View Menu Toggle

```
1. User clicks View → Servers
   ↓
2. MenuAction::View(ViewMenuItem::Servers)
   ↓
3. DatabaseIDE::update()
   ├─ view_registry.toggle(ComponentId::ServerList)
   ├─ If ServerList was active_tab:
   │  ├─ Query enabled_views(LeftPanel)
   │  └─ Set active_tab to first remaining (or None)
   └─ Trigger re-render
   ↓
4. DatabaseIDE::view()
   ├─ MenuBar renders checkmarks based on registry.is_enabled()
   ├─ LeftPanel.view() queries enabled_views(LeftPanel)
   ├─ Tabs rendered from enabled views only
   └─ Active component's view() method called
```

### Tab Selection

```
1. User clicks "Tables" tab
   ↓
2. Message::SelectTab(RegionId::LeftPanel, ComponentId::TableList)
   ↓
3. DatabaseIDE::update()
   └─ left_panel.set_active_component(ComponentId::TableList)
   ↓
4. DatabaseIDE::view()
   ├─ LeftPanel.view() queries enabled_views(LeftPanel)
   ├─ Renders tabs (Tables highlighted as active)
   └─ Calls TableList.view() for content
```

### Component Action

```
1. User clicks "New Connection" in ServerList
   ↓
2. Component emits ComponentAction::ServerList(ServerListAction::NewConnection)
   ↓
3. Callback maps to Message::ComponentAction(...)
   ↓
4. DatabaseIDE::update()
   └─ Match on ComponentAction::ServerList variant
   └─ Handle NewConnection (e.g., open connection dialog)
```

## Adding New Components

### Step-by-Step Guide

**Step 1: Add ComponentId variant**

```rust
// In components/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    ServerList,
    TableList,
    Properties,
    Editor,
    ConnectionForm,
    QueryResults,  // ← New component
}

impl ComponentId {
    pub fn name(&self) -> &'static str {
        match self {
            // ... existing
            ComponentId::QueryResults => "Query Results",
        }
    }
}
```

**Step 2: Add ComponentAction variant**

```rust
// In components/mod.rs
#[derive(Debug, Clone)]
pub enum ComponentAction {
    ServerList(ServerListAction),
    TableList(TableListAction),
    Properties(PropertiesAction),
    Editor(EditorAction),
    ConnectionForm(ConnectionFormAction),
    QueryResults(QueryResultsAction),  // ← New action
}

#[derive(Debug, Clone)]
pub enum QueryResultsAction {
    ExportCsv,
    CopyToClipboard,
    SortColumn(usize),
}
```

**Step 3: Implement Component trait**

```rust
// In components/query_results.rs
pub struct QueryResultsComponent;

impl Component for QueryResultsComponent {
    fn id(&self) -> ComponentId {
        ComponentId::QueryResults
    }

    fn view<'a, Message: 'a + Clone>(
        &'a self,
        theme: ThemeColors,
        on_action: impl Fn(ComponentAction) -> Message + 'a,
    ) -> Element<'a, Message> {
        // Component implementation
        let export_button = button(text("Export CSV"))
            .on_press(on_action(
                ComponentAction::QueryResults(
                    QueryResultsAction::ExportCsv
                )
            ));

        container(column![export_button]).into()
    }
}
```

**Step 4: Register in DatabaseIDE**

```rust
// In main.rs or lib.rs (DatabaseIDE::new)
view_registry.register(
    Box::new(QueryResultsComponent),
    RegionId::MainPanel  // Choose appropriate region
);
```

**Step 5: Add to View menu (optional)**

```rust
// In menu_bar.rs
pub enum ViewMenuItem {
    ToggleLeftPanel,
    Servers,
    Tables,
    Properties,
    QueryEditor,
    ConnectionManager,
    QueryResults,  // ← New menu item
}

impl ViewMenuItem {
    pub fn as_component_id(&self) -> Option<ComponentId> {
        match self {
            // ... existing
            ViewMenuItem::QueryResults => Some(ComponentId::QueryResults),
        }
    }
}
```

**Step 6: Handle actions in update()**

```rust
// In DatabaseIDE::update()
match message {
    Message::ComponentAction(action) => {
        match action {
            // ... existing
            ComponentAction::QueryResults(results_action) => {
                match results_action {
                    QueryResultsAction::ExportCsv => {
                        // Handle export
                    }
                    QueryResultsAction::CopyToClipboard => {
                        // Handle copy
                    }
                    QueryResultsAction::SortColumn(col) => {
                        // Handle sort
                    }
                }
            }
        }
        Task::none()
    }
}
```

### Component Checklist

When adding a new component, ensure:

- [ ] ComponentId variant added
- [ ] ComponentId::name() returns display name
- [ ] ComponentAction variant added (if component has actions)
- [ ] Component-specific action enum defined (if needed)
- [ ] Component struct implements Component trait
- [ ] Component registered in DatabaseIDE::new()
- [ ] ViewMenuItem variant added (if toggle-able)
- [ ] ViewMenuItem::as_component_id() mapping added
- [ ] ComponentAction handling in DatabaseIDE::update()

## File Organization

```
rusty-app/src/
├── main.rs                  // Application entry, DatabaseIDE
├── lib.rs                   // Public API
├── theme.rs                 // ThemeColors
├── menu_bar.rs              // MenuBar, ViewMenuItem, MenuAction
├── views.rs                 // View, ViewRegistry, RegionId
├── regions.rs               // Region trait, LeftPanelRegion, MainPanelRegion
├── components/
│   ├── mod.rs               // Component trait, ComponentId, ComponentAction
│   ├── server_list.rs       // ServerListComponent, ServerListAction
│   ├── table_list.rs        // TableListComponent, TableListAction
│   ├── properties.rs        // PropertiesComponent, PropertiesAction
│   ├── editor.rs            // EditorComponent, EditorAction
│   └── connection_form.rs   // ConnectionFormComponent, ConnectionFormAction
```

**Module Responsibilities:**

| Module | Exports | Purpose |
|--------|---------|---------|
| `views.rs` | View, ViewRegistry, RegionId | View management layer |
| `regions.rs` | Region trait, LeftPanelRegion, MainPanelRegion | Region implementations |
| `components/mod.rs` | Component trait, ComponentId, ComponentAction | Component abstraction |
| `components/*.rs` | Individual component implementations | Concrete components |

## Future Extensions

### Drag-and-Drop Docking

To support drag-and-drop docking in the future:

1. Make View.region mutable:
   ```rust
   impl View {
       pub fn set_region(&mut self, region: RegionId) {
           self.region = region;
       }
   }
   ```

2. Add drag state to regions:
   ```rust
   pub struct LeftPanelRegion {
       active_tab: Option<ComponentId>,
       dragging: Option<ComponentId>,
       // ...
   }
   ```

3. Add ViewRegistry method:
   ```rust
   impl ViewRegistry {
       pub fn move_to_region(&mut self, id: ComponentId, new_region: RegionId) {
           if let Some(view) = self.views.get_mut(&id) {
               view.set_region(new_region);
           }
       }
   }
   ```

### Floating Windows

To support floating component windows:

1. Add RegionId variant:
   ```rust
   pub enum RegionId {
       LeftPanel,
       MainPanel,
       FloatingWindow(usize),  // Window ID
   }
   ```

2. Create FloatingWindowRegion:
   ```rust
   pub struct FloatingWindowRegion {
       window_id: usize,
       component: ComponentId,
       position: (f32, f32),
       size: (f32, f32),
   }
   ```

### Saved Layouts

To support saving/loading layouts:

1. Add serialization to View:
   ```rust
   #[derive(Serialize, Deserialize)]
   pub struct ViewLayout {
       pub component_id: ComponentId,
       pub enabled: bool,
       pub region: RegionId,
   }

   impl ViewRegistry {
       pub fn export_layout(&self) -> Vec<ViewLayout> { ... }
       pub fn import_layout(&mut self, layout: Vec<ViewLayout>) { ... }
   }
   ```

2. Add region state serialization:
   ```rust
   #[derive(Serialize, Deserialize)]
   pub struct RegionLayout {
       pub active_tab: Option<ComponentId>,
       pub width: Option<f32>,
       pub visible: Option<bool>,
   }
   ```

## Summary

### Key Architecture Principles

1. **Separation of Concerns**:
   - Components: Stateless rendering
   - Views: Toggle-able visibility
   - ViewRegistry: Centralized state management
   - Regions: Layout and composition

2. **Type Safety**:
   - Enums for IDs (ComponentId, RegionId)
   - Enum dispatch for actions (ComponentAction)
   - No string-based lookups

3. **Single Source of Truth**:
   - ViewRegistry owns enabled state
   - Regions manage active tab state
   - No duplicate state

4. **Dynamic Updates**:
   - Render-time queries to ViewRegistry
   - Tabs generated from enabled views
   - View menu reflects current state

5. **Extensibility**:
   - Adding components: add variants, implement trait
   - Adding regions: implement Region trait
   - Adding actions: extend ComponentAction enum

### Implementation Status

- [x] Architecture designed (Tasks 5vf.1.1 - 5vf.1.5)
- [ ] View System Implementation (Feature 5vf.2)
- [ ] View Menu Integration (Feature 5vf.3)
- [ ] Dynamic Left Panel (Feature 5vf.4)
- [ ] Component Refactoring (Feature 5vf.5)
- [ ] Region System (Feature 5vf.6)

### Related Documentation

- **CLAUDE.md**: Project overview and development guidelines
- **README.md**: User-facing documentation
- **Design Documents** (in /tmp during discovery):
  - `/tmp/iced_patterns_research.md`: Pattern research
  - `/tmp/component_trait_design.md`: Component trait specification
  - `/tmp/view_registry_design.md`: View/ViewRegistry specification
  - `/tmp/region_trait_design.md`: Region trait specification

---

**Maintained by**: Claude Code
**Architecture Version**: 1.0
**Epic**: rusty-app-5vf (Component-Based Docking Architecture)
