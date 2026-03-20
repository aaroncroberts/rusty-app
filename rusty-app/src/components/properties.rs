//! Properties component for displaying object details

use crate::components::{Component, ComponentAction, ComponentId, PropertiesAction};
use crate::theme::ThemeColors;
use iced::widget::{column, container, scrollable, text};
use iced::{Element, Fill};

/// Property key-value pair
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub key: String,
    pub value: String,
}

/// Properties component with internal property list
#[derive(Debug, Clone)]
pub struct PropertiesComponent {
    properties: Vec<Property>,
    selected_object: Option<String>,
}

impl PropertiesComponent {
    /// Create a new properties component
    pub fn new() -> Self {
        Self {
            properties: Vec::new(),
            selected_object: None,
        }
    }

    /// Get the current properties
    pub fn properties(&self) -> &[Property] {
        &self.properties
    }

    /// Set the properties list
    pub fn set_properties(&mut self, properties: Vec<Property>) {
        self.properties = properties;
    }

    /// Set the selected object name
    pub fn set_selected_object(&mut self, object: Option<String>) {
        let is_none = object.is_none();
        self.selected_object = object;
        // Clear properties when object changes
        if is_none {
            self.properties.clear();
        }
    }

    /// Get the selected object name
    pub fn selected_object(&self) -> Option<&str> {
        self.selected_object.as_deref()
    }

    /// Add a property
    pub fn add_property(&mut self, key: String, value: String) {
        self.properties.push(Property { key, value });
    }

    /// Clear all properties
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// Update a property value by key
    pub fn update_property(&mut self, key: &str, value: String) -> bool {
        if let Some(prop) = self.properties.iter_mut().find(|p| p.key == key) {
            prop.value = value;
            true
        } else {
            false
        }
    }

    /// Update the component based on an action
    pub fn update(&mut self, action: PropertiesAction) {
        match action {
            PropertiesAction::UpdateProperty(key, value) => {
                self.update_property(&key, value);
            }
        }
    }
}

impl Default for PropertiesComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for PropertiesComponent {
    fn id(&self) -> ComponentId {
        ComponentId::Properties
    }

    fn view(&self, theme: ThemeColors) -> Element<ComponentAction> {
        let mut content_col = column![
            text("Properties").size(12).color(theme.text),
            text("─────────").size(10).color(theme.border),
        ]
        .spacing(10);

        if self.selected_object.is_none() {
            content_col = content_col.push(
                text("(Select an object)")
                    .size(11)
                    .color(theme.text_secondary),
            );
        } else if self.properties.is_empty() {
            content_col =
                content_col.push(text("(No properties)").size(11).color(theme.text_secondary));
        } else {
            for prop in &self.properties {
                content_col = content_col.push(
                    column![
                        text(&prop.key).size(10).color(theme.text_secondary),
                        text(&prop.value).size(11).color(theme.text),
                    ]
                    .spacing(2),
                );
            }
        }

        container(scrollable(content_col.padding(15)).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_properties_component_new() {
        let component = PropertiesComponent::new();
        assert_eq!(component.id(), ComponentId::Properties);
        assert_eq!(component.properties().len(), 0);
        assert_eq!(component.selected_object(), None);
    }

    #[test]
    fn test_properties_set_properties() {
        let mut component = PropertiesComponent::new();
        let properties = vec![
            Property {
                key: "Name".to_string(),
                value: "users".to_string(),
            },
            Property {
                key: "Type".to_string(),
                value: "Table".to_string(),
            },
        ];

        component.set_properties(properties);
        assert_eq!(component.properties().len(), 2);
        assert_eq!(component.properties()[0].key, "Name");
    }

    #[test]
    fn test_properties_add_property() {
        let mut component = PropertiesComponent::new();

        component.add_property("Schema".to_string(), "public".to_string());
        assert_eq!(component.properties().len(), 1);
        assert_eq!(component.properties()[0].key, "Schema");
        assert_eq!(component.properties()[0].value, "public");
    }

    #[test]
    fn test_properties_update_property() {
        let mut component = PropertiesComponent::new();
        component.add_property("Status".to_string(), "Active".to_string());

        assert!(component.update_property("Status", "Inactive".to_string()));
        assert_eq!(component.properties()[0].value, "Inactive");

        assert!(!component.update_property("NonExistent", "Value".to_string()));
    }

    #[test]
    fn test_properties_clear() {
        let mut component = PropertiesComponent::new();
        component.add_property("Key1".to_string(), "Value1".to_string());
        component.add_property("Key2".to_string(), "Value2".to_string());

        assert_eq!(component.properties().len(), 2);

        component.clear();
        assert_eq!(component.properties().len(), 0);
    }

    #[test]
    fn test_properties_set_selected_object() {
        let mut component = PropertiesComponent::new();

        component.set_selected_object(Some("table1".to_string()));
        assert_eq!(component.selected_object(), Some("table1"));

        component.set_selected_object(None);
        assert_eq!(component.selected_object(), None);
    }

    #[test]
    fn test_properties_clear_on_object_change() {
        let mut component = PropertiesComponent::new();
        component.set_selected_object(Some("table1".to_string()));
        component.add_property("Name".to_string(), "table1".to_string());

        assert_eq!(component.properties().len(), 1);

        component.set_selected_object(None);
        assert_eq!(component.properties().len(), 0);
    }
}
