//! Results view component for displaying query results

use crate::components::{Component, ComponentAction, ComponentId};
use crate::theme::ThemeColors;
use iced::widget::{column, container, text};
use iced::{Element, Fill};

/// Results view component that displays query execution results
///
/// This component wraps the result grid and provides a toggleable view
/// for displaying query results in the main panel.
#[derive(Debug, Clone)]
pub struct ResultsViewComponent {
    // Currently minimal - future: pagination, filtering, sorting state
}

impl ResultsViewComponent {
    /// Create a new results view component
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ResultsViewComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ResultsViewComponent {
    fn id(&self) -> ComponentId {
        ComponentId::ResultsView
    }

    fn view(&self, theme: ThemeColors) -> Element<ComponentAction> {
        // Placeholder view - actual result grid rendering will be integrated later
        let content = column![text("Results").size(14).color(theme.text),]
            .spacing(10)
            .padding(15);

        container(content)
            .width(Fill)
            .height(Fill)
            .style(move |_theme| container::Style {
                background: Some(theme.background.into()),
                ..Default::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_results_view_component_new() {
        let component = ResultsViewComponent::new();
        assert_eq!(component.id(), ComponentId::ResultsView);
    }

    #[test]
    fn test_results_view_component_default() {
        let component = ResultsViewComponent::default();
        assert_eq!(component.id(), ComponentId::ResultsView);
    }

    #[test]
    fn test_results_view_component_id() {
        let component = ResultsViewComponent::new();
        assert_eq!(component.id(), ComponentId::ResultsView);
    }

    #[test]
    fn test_results_view_component_title() {
        let component = ResultsViewComponent::new();
        assert_eq!(component.title(), "Results");
    }

    #[test]
    fn test_results_view_component_view() {
        let theme = ThemeColors::dark();
        let component = ResultsViewComponent::new();

        // Should not panic
        let _view = component.view(theme);
    }

    #[test]
    fn test_results_view_component_cloneable() {
        let component = ResultsViewComponent::new();
        let cloned = component.clone();

        assert_eq!(component.id(), cloned.id());
    }

    #[test]
    fn test_results_view_component_debug() {
        let component = ResultsViewComponent::new();
        let debug_str = format!("{:?}", component);
        assert!(debug_str.contains("ResultsViewComponent"));
    }
}
