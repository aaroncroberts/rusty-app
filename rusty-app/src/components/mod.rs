//! Component system for dynamic UI composition.
//!
//! This module provides:
//! - `ComponentId`: Type-safe component identification
//! - `ComponentAction`: Unified action type for all components
//! - `Component`: Trait for UI components

use crate::theme::ThemeColors;
use iced::Element;

pub mod connection_form;
pub mod editor;
pub mod properties;
pub mod results_view;
pub mod server_list;
pub mod table_list;

pub use connection_form::{ConnectionFormComponent, ConnectionFormData};
pub use editor::{EditorComponent, QueryTab};
pub use properties::{PropertiesComponent, Property};
pub use results_view::ResultsViewComponent;
pub use server_list::ServerListComponent;
pub use table_list::{TableListComponent, TableMetadata};

/// Unique identifier for each component type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    ServerList,
    TableList,
    Properties,
    Editor,
    ConnectionForm,
    ResultsView,
}

impl ComponentId {
    /// Get all component IDs
    pub fn all() -> &'static [ComponentId] {
        &[
            ComponentId::ServerList,
            ComponentId::TableList,
            ComponentId::Properties,
            ComponentId::Editor,
            ComponentId::ConnectionForm,
            ComponentId::ResultsView,
        ]
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ComponentId::ServerList => "Servers",
            ComponentId::TableList => "Tables",
            ComponentId::Properties => "Properties",
            ComponentId::Editor => "Query Editor",
            ComponentId::ConnectionForm => "Connection Manager",
            ComponentId::ResultsView => "Results",
        }
    }
}

/// Unified action type for all components
#[derive(Debug, Clone)]
pub enum ComponentAction {
    ServerList(ServerListAction),
    TableList(TableListAction),
    Properties(PropertiesAction),
    Editor(EditorAction),
    ConnectionForm(ConnectionFormAction),
    ResultsView(ResultsViewAction),
}

/// Actions for the server list component
#[derive(Debug, Clone)]
pub enum ServerListAction {
    SelectServer(String),
    NewConnection,
    RefreshList,
}

/// Actions for the table list component
#[derive(Debug, Clone)]
pub enum TableListAction {
    SelectTable(String),
    RefreshTables,
}

/// Actions for the properties component
#[derive(Debug, Clone)]
pub enum PropertiesAction {
    UpdateProperty(String, String),
}

/// Actions for the editor component
#[derive(Debug, Clone)]
pub enum EditorAction {
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    QueryChanged(String),
    ExecuteQuery,
}

/// Actions for the connection form component
#[derive(Debug, Clone)]
pub enum ConnectionFormAction {
    NameChanged(String),
    DbTypeChanged(arni::DatabaseType),
    HostChanged(String),
    PortChanged(String),
    DatabaseChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    FilePathChanged(String),
    TestConnection,
    Save,
    Cancel,
}

/// Actions for the results view component
#[derive(Debug, Clone)]
pub enum ResultsViewAction {
    // ResultsView is read-only display, no actions needed yet
    // Future: pagination, sorting, filtering actions
}

/// Trait for UI components that can be rendered dynamically
pub trait Component: Send {
    /// Unique identifier for this component
    fn id(&self) -> ComponentId;

    /// Display name for tabs and menus
    fn title(&self) -> &str {
        self.id().name()
    }

    /// Render the component
    ///
    /// # Parameters
    /// - `theme`: Application theme colors
    ///
    /// # Returns
    /// Element that emits ComponentAction messages
    fn view(&self, theme: ThemeColors) -> Element<'_, ComponentAction>;
}

/// Map a component's Element<ComponentAction> to Element<Message>
///
/// This helper function allows components to be composed into any message context
/// by providing a mapping function from ComponentAction to the parent's Message type.
pub fn map_component_view<'a, Message: 'a + Clone>(
    element: Element<'a, ComponentAction>,
    on_action: impl Fn(ComponentAction) -> Message + 'a,
) -> Element<'a, Message> {
    element.map(move |action: ComponentAction| on_action(action))
}
