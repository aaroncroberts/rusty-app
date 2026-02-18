//! View management system for the component-based docking architecture.
//!
//! This module provides:
//! - `RegionId`: Type-safe region identification
//! - `View`: Component wrapper with enabled state and region assignment
//! - `ViewRegistry`: Centralized view management

use std::collections::HashMap;
use crate::components::{Component, ComponentId};

/// Identifies dockable regions in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionId {
    LeftPanel,
    MainPanel,
}

impl RegionId {
    /// Get all region IDs
    pub fn all() -> &'static [RegionId] {
        &[RegionId::LeftPanel, RegionId::MainPanel]
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            RegionId::LeftPanel => "Left Panel",
            RegionId::MainPanel => "Main Panel",
        }
    }
}

/// A component with visibility state
///
/// Views wrap components and track whether they're currently enabled.
/// Only enabled views are rendered in their assigned region.
pub struct View {
    /// The component to render
    component: Box<dyn Component>,

    /// Whether this view is currently enabled
    enabled: bool,

    /// Which region this view belongs to
    region: RegionId,
}

impl View {
    /// Create a new view for a component
    pub fn new(component: Box<dyn Component>, region: RegionId) -> Self {
        Self {
            component,
            enabled: true, // Default to enabled
            region,
        }
    }

    /// Check if this view is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable this view
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable this view
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Toggle this view's enabled state
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Get the region this view belongs to
    pub fn region(&self) -> RegionId {
        self.region
    }

    /// Get the underlying component
    pub fn component(&self) -> &dyn Component {
        self.component.as_ref()
    }
}

/// Centralized registry for managing views
///
/// Tracks all application views and their enabled/disabled state.
/// Provides queries for enabled views per region.
pub struct ViewRegistry {
    /// Map from ComponentId to View
    views: HashMap<ComponentId, View>,
}

impl ViewRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    /// Register a component as a view
    pub fn register(&mut self, component: Box<dyn Component>, region: RegionId) {
        let id = component.id();
        self.views.insert(id, View::new(component, region));
    }

    /// Enable a view by component ID
    pub fn enable(&mut self, id: ComponentId) -> bool {
        if let Some(view) = self.views.get_mut(&id) {
            view.enable();
            true
        } else {
            false
        }
    }

    /// Disable a view by component ID
    pub fn disable(&mut self, id: ComponentId) -> bool {
        if let Some(view) = self.views.get_mut(&id) {
            view.disable();
            true
        } else {
            false
        }
    }

    /// Toggle a view's enabled state
    pub fn toggle(&mut self, id: ComponentId) -> bool {
        if let Some(view) = self.views.get_mut(&id) {
            view.toggle();
            true
        } else {
            false
        }
    }

    /// Check if a view is enabled
    pub fn is_enabled(&self, id: ComponentId) -> bool {
        self.views
            .get(&id)
            .map(|view| view.is_enabled())
            .unwrap_or(false)
    }

    /// Get all enabled views for a specific region
    pub fn enabled_views(&self, region: RegionId) -> Vec<&View> {
        self.views
            .values()
            .filter(|view| view.region() == region && view.is_enabled())
            .collect()
    }

    /// Get all views for a specific region (enabled or disabled)
    pub fn region_views(&self, region: RegionId) -> Vec<&View> {
        self.views
            .values()
            .filter(|view| view.region() == region)
            .collect()
    }

    /// Get a view by component ID
    pub fn get_view(&self, id: ComponentId) -> Option<&View> {
        self.views.get(&id)
    }

    /// Get all component IDs
    pub fn all_ids(&self) -> Vec<ComponentId> {
        self.views.keys().copied().collect()
    }
}

impl Default for ViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Component, ComponentId, ComponentAction};
    use crate::theme::ThemeColors;
    use iced::Element;

    // Mock component for testing
    struct MockComponent {
        id: ComponentId,
        title: &'static str,
    }

    impl MockComponent {
        fn new(id: ComponentId, title: &'static str) -> Self {
            Self { id, title }
        }
    }

    impl Component for MockComponent {
        fn id(&self) -> ComponentId {
            self.id
        }

        fn title(&self) -> &str {
            self.title
        }

        fn view(&self, _theme: ThemeColors) -> Element<ComponentAction> {
            iced::widget::text(self.title).into()
        }
    }

    #[test]
    fn test_new_registry_is_empty() {
        let registry = ViewRegistry::new();
        assert_eq!(registry.all_ids().len(), 0);
    }

    #[test]
    fn test_register_component() {
        let mut registry = ViewRegistry::new();
        let component = Box::new(MockComponent::new(ComponentId::ServerList, "Servers"));

        registry.register(component, RegionId::LeftPanel);

        assert_eq!(registry.all_ids().len(), 1);
        assert!(registry.all_ids().contains(&ComponentId::ServerList));
    }

    #[test]
    fn test_register_multiple_components() {
        let mut registry = ViewRegistry::new();

        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );
        registry.register(
            Box::new(MockComponent::new(ComponentId::TableList, "Tables")),
            RegionId::LeftPanel,
        );
        registry.register(
            Box::new(MockComponent::new(ComponentId::Editor, "Editor")),
            RegionId::MainPanel,
        );

        assert_eq!(registry.all_ids().len(), 3);
    }

    #[test]
    fn test_component_enabled_by_default() {
        let mut registry = ViewRegistry::new();
        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );

        assert!(registry.is_enabled(ComponentId::ServerList));
    }

    #[test]
    fn test_disable_component() {
        let mut registry = ViewRegistry::new();
        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );

        assert!(registry.disable(ComponentId::ServerList));
        assert!(!registry.is_enabled(ComponentId::ServerList));
    }

    #[test]
    fn test_enable_component() {
        let mut registry = ViewRegistry::new();
        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );

        registry.disable(ComponentId::ServerList);
        assert!(!registry.is_enabled(ComponentId::ServerList));

        assert!(registry.enable(ComponentId::ServerList));
        assert!(registry.is_enabled(ComponentId::ServerList));
    }

    #[test]
    fn test_toggle_component() {
        let mut registry = ViewRegistry::new();
        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );

        // Initially enabled
        assert!(registry.is_enabled(ComponentId::ServerList));

        // Toggle to disabled
        assert!(registry.toggle(ComponentId::ServerList));
        assert!(!registry.is_enabled(ComponentId::ServerList));

        // Toggle back to enabled
        assert!(registry.toggle(ComponentId::ServerList));
        assert!(registry.is_enabled(ComponentId::ServerList));
    }

    #[test]
    fn test_enable_nonexistent_component_returns_false() {
        let mut registry = ViewRegistry::new();
        assert!(!registry.enable(ComponentId::ServerList));
    }

    #[test]
    fn test_is_enabled_nonexistent_component_returns_false() {
        let registry = ViewRegistry::new();
        assert!(!registry.is_enabled(ComponentId::ServerList));
    }

    #[test]
    fn test_enabled_views_filters_by_region() {
        let mut registry = ViewRegistry::new();

        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );
        registry.register(
            Box::new(MockComponent::new(ComponentId::TableList, "Tables")),
            RegionId::LeftPanel,
        );
        registry.register(
            Box::new(MockComponent::new(ComponentId::Editor, "Editor")),
            RegionId::MainPanel,
        );

        let left_panel_views = registry.enabled_views(RegionId::LeftPanel);
        assert_eq!(left_panel_views.len(), 2);

        let main_panel_views = registry.enabled_views(RegionId::MainPanel);
        assert_eq!(main_panel_views.len(), 1);
    }

    #[test]
    fn test_enabled_views_filters_by_enabled_state() {
        let mut registry = ViewRegistry::new();

        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );
        registry.register(
            Box::new(MockComponent::new(ComponentId::TableList, "Tables")),
            RegionId::LeftPanel,
        );

        // Disable one component
        registry.disable(ComponentId::TableList);

        let enabled_views = registry.enabled_views(RegionId::LeftPanel);
        assert_eq!(enabled_views.len(), 1);
        assert_eq!(enabled_views[0].component().id(), ComponentId::ServerList);
    }

    #[test]
    fn test_region_views_includes_disabled() {
        let mut registry = ViewRegistry::new();

        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );
        registry.register(
            Box::new(MockComponent::new(ComponentId::TableList, "Tables")),
            RegionId::LeftPanel,
        );

        // Disable one component
        registry.disable(ComponentId::TableList);

        // region_views should return all views (enabled or disabled)
        let all_views = registry.region_views(RegionId::LeftPanel);
        assert_eq!(all_views.len(), 2);
    }

    #[test]
    fn test_get_view() {
        let mut registry = ViewRegistry::new();

        registry.register(
            Box::new(MockComponent::new(ComponentId::ServerList, "Servers")),
            RegionId::LeftPanel,
        );

        let view = registry.get_view(ComponentId::ServerList);
        assert!(view.is_some());
        assert_eq!(view.unwrap().component().id(), ComponentId::ServerList);
    }

    #[test]
    fn test_get_view_nonexistent() {
        let registry = ViewRegistry::new();
        let view = registry.get_view(ComponentId::ServerList);
        assert!(view.is_none());
    }
}
