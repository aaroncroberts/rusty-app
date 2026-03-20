use iced::event::Event;
use iced::mouse;
use iced::widget::{column, container, row};
use iced::{Element, Fill, Subscription, Task, Theme};
use rusty_app::adapter_selector::{AdapterSelector, AdapterSelectorMessage};
use rusty_app::components::{
    ComponentAction, ComponentId, PropertiesComponent, ServerListComponent, TableListComponent,
};
use rusty_app::connection_form::{ConnectionForm, ConnectionFormData, ConnectionFormMessage};
use rusty_app::connection_manager::{ActiveConnection, ConnectionManager};
use rusty_app::container::converter::sync_connections_with_containers;
use rusty_app::container::ContainerManager;
use rusty_app::left_panel::{self, LeftPanel};
use rusty_app::main_panel::{MainPanel, TabId};
use rusty_app::menu_bar::{MenuAction, MenuBar, MenuItem};
use rusty_app::mongodb_connection_form::{
    MongoDBConnectionForm, MongoDBConnectionFormData, MongoDBConnectionFormMessage,
};
use rusty_app::mysql_connection_form::{
    MySQLConnectionForm, MySQLConnectionFormData, MySQLConnectionFormMessage,
};
use rusty_app::oracle_connection_form::{
    OracleConnectionForm, OracleConnectionFormData, OracleConnectionFormMessage,
};
use rusty_app::postgres_connection_form::{
    PostgresConnectionForm, PostgresConnectionFormData, PostgresConnectionFormMessage,
};
use rusty_app::query_editor::QueryEditorMessage;
use rusty_app::settings::{logging::build_logging_config, SettingsManager};
use rusty_app::settings_editor::{SettingsEditor, SettingsEditorData, SettingsEditorMessage};
use rusty_app::sqlite_connection_form::{
    SQLiteConnectionForm, SQLiteConnectionFormData, SQLiteConnectionFormMessage,
};
use rusty_app::sqlserver_connection_form::{
    SQLServerConnectionForm, SQLServerConnectionFormData, SQLServerConnectionFormMessage,
};
use rusty_app::status_bar::{ConnectionStatus, StatusBar};
use rusty_app::theme::ThemeColors;
use rusty_app::views::{RegionId, ViewRegistry};
use arni::{ConnectionConfig, DatabaseType};

use arni::QueryResult;
use std::collections::HashMap;
use tracing::{info, warn};

pub fn main() -> iced::Result {
    // Load settings and configure logging
    let settings_manager =
        SettingsManager::new("~/.rusty-app").expect("Failed to initialize settings manager");

    let logging_config = build_logging_config(&settings_manager.settings().logging)
        .expect("Invalid logging configuration");

    logging_config
        .apply()
        .expect("Failed to initialize logging");

    info!(version = env!("CARGO_PKG_VERSION"), "rusty-app starting");

    iced::application("rusty-app: the ide", DatabaseIDE::update, DatabaseIDE::view)
        .subscription(DatabaseIDE::subscription)
        .theme(|_| Theme::TokyoNightStorm)
        .window_size((1280.0, 800.0))
        .run()
}

struct DatabaseIDE {
    theme: ThemeColors,
    menu_bar: MenuBar,
    left_panel: LeftPanel,
    main_panel: MainPanel,
    status_bar: StatusBar,
    connection_form: ConnectionForm,
    panel_width: f32,
    active_component: Option<ComponentId>,
    is_resizing: bool,
    last_mouse_x: Option<f32>,
    connection_status: ConnectionStatus,
    container_manager: ContainerManager,
    settings_manager: SettingsManager,
    saved_connections: Vec<ConnectionConfig>,
    showing_connection_form: bool,
    connection_form_data: ConnectionFormData,
    testing_connection: bool,
    test_result: Option<Result<(), String>>,
    open_menu: Option<MenuItem>,
    show_left_panel: bool,
    view_registry: ViewRegistry,
    showing_settings_editor: bool,
    settings_editor: SettingsEditor,
    settings_editor_data: SettingsEditorData,
    settings_validation_error: Option<String>,
    settings_success_message: Option<String>,
    // Adapter-specific connection forms
    showing_adapter_selector: bool,
    adapter_selector: AdapterSelector,
    selected_adapter: Option<DatabaseType>,
    postgres_form: PostgresConnectionForm,
    postgres_form_data: PostgresConnectionFormData,
    postgres_testing: bool,
    postgres_test_result: Option<Result<(), String>>,
    mysql_form: MySQLConnectionForm,
    mysql_form_data: MySQLConnectionFormData,
    mysql_testing: bool,
    mysql_test_result: Option<Result<(), String>>,
    sqlite_form: SQLiteConnectionForm,
    sqlite_form_data: SQLiteConnectionFormData,
    sqlite_testing: bool,
    sqlite_test_result: Option<Result<(), String>>,
    mongodb_form: MongoDBConnectionForm,
    mongodb_form_data: MongoDBConnectionFormData,
    mongodb_testing: bool,
    mongodb_test_result: Option<Result<(), String>>,
    sqlserver_form: SQLServerConnectionForm,
    sqlserver_form_data: SQLServerConnectionFormData,
    sqlserver_testing: bool,
    sqlserver_test_result: Option<Result<(), String>>,
    oracle_form: OracleConnectionForm,
    oracle_form_data: OracleConnectionFormData,
    oracle_testing: bool,
    oracle_test_result: Option<Result<(), String>>,
    // Connection management
    connection_manager: ConnectionManager,
    pending_connection_config: Option<(ConnectionConfig, Option<String>)>, // config and password for connection being established
    // Query execution
    executing_query: bool,
    query_results: HashMap<TabId, Option<QueryResult>>,
    query_errors: HashMap<TabId, Option<String>>,
}

impl Default for DatabaseIDE {
    fn default() -> Self {
        // Initialize SettingsManager
        let mut settings_manager =
            SettingsManager::new("~/.rusty-app").expect("Failed to initialize settings manager");

        // Load UI preferences from settings (clone to avoid borrow issues)
        let ui_prefs = settings_manager.settings().ui_preferences.clone();

        // Initialize settings editor data from manager
        let settings_editor_data = SettingsEditorData::from_manager(&settings_manager);

        // Set theme based on settings
        let theme = if ui_prefs.theme == "dark" {
            ThemeColors::dark()
        } else {
            ThemeColors::dark() // TODO: Add ThemeColors::light()
        };

        let mut main_panel = MainPanel::new(theme);

        // Create an initial tab
        main_panel.add_tab("Query 1".to_string());

        // Initialize ContainerManager
        let container_manager = ContainerManager::new("rusty-data");

        // Load saved connections from settings.toml
        let mut saved_connections = settings_manager.settings().connections.clone();

        info!(
            count = saved_connections.len(),
            "Loaded saved connections from settings"
        );

        // Sync connections with running containers
        if container_manager.is_podman_available() {
            match container_manager.list_containers() {
                Ok(containers) => {
                    let running_containers: Vec<_> = containers
                        .iter()
                        .filter(|c| c.is_running())
                        .cloned()
                        .collect();

                    info!(count = running_containers.len(), "Found running containers");

                    // Sync connections with container state
                    saved_connections =
                        sync_connections_with_containers(&running_containers, &saved_connections);

                    // Save updated connections to settings.toml
                    settings_manager.settings_mut().connections = saved_connections.clone();
                    if let Err(e) = settings_manager.save() {
                        warn!("Failed to save synced connections: {}", e);
                    } else {
                        info!(
                            count = saved_connections.len(),
                            "Synced connections with container state"
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to list containers: {}", e);
                }
            }
        } else {
            info!("Podman not available, skipping container sync");
        }

        // Create and register components in ViewRegistry
        let mut view_registry = ViewRegistry::new();

        // Create ServerList component with loaded connections
        let mut server_list = ServerListComponent::new();
        server_list.set_connections(saved_connections.clone());
        view_registry.register(Box::new(server_list), RegionId::LeftPanel);

        // Create TableList component
        let table_list = TableListComponent::new();
        view_registry.register(Box::new(table_list), RegionId::LeftPanel);

        // Create Properties component
        let properties = PropertiesComponent::new();
        view_registry.register(Box::new(properties), RegionId::LeftPanel);

        Self {
            theme,
            menu_bar: MenuBar::new(theme),
            left_panel: LeftPanel::new(theme),
            main_panel,
            status_bar: StatusBar::new(theme),
            connection_form: ConnectionForm::new(theme),
            panel_width: ui_prefs.panel_width as f32,
            active_component: match ui_prefs.active_component.as_str() {
                "server_list" => Some(ComponentId::ServerList),
                "table_list" => Some(ComponentId::TableList),
                "properties" => Some(ComponentId::Properties),
                _ => Some(ComponentId::ServerList),
            },
            is_resizing: false,
            last_mouse_x: None,
            connection_status: ConnectionStatus::default(),
            container_manager,
            settings_manager,
            saved_connections,
            showing_connection_form: false,
            connection_form_data: ConnectionFormData::new(),
            testing_connection: false,
            test_result: None,
            open_menu: None,
            show_left_panel: ui_prefs.show_left_panel,
            view_registry,
            showing_settings_editor: false,
            settings_editor: SettingsEditor::new(theme),
            settings_editor_data,
            settings_validation_error: None,
            settings_success_message: None,
            // Adapter-specific connection forms
            showing_adapter_selector: false,
            adapter_selector: AdapterSelector::new(theme),
            selected_adapter: None,
            postgres_form: PostgresConnectionForm::new(theme),
            postgres_form_data: PostgresConnectionFormData::new(),
            postgres_testing: false,
            postgres_test_result: None,
            mysql_form: MySQLConnectionForm::new(theme),
            mysql_form_data: MySQLConnectionFormData::new(),
            mysql_testing: false,
            mysql_test_result: None,
            sqlite_form: SQLiteConnectionForm::new(theme),
            sqlite_form_data: SQLiteConnectionFormData::new(),
            sqlite_testing: false,
            sqlite_test_result: None,
            mongodb_form: MongoDBConnectionForm::new(theme),
            mongodb_form_data: MongoDBConnectionFormData::new(),
            mongodb_testing: false,
            mongodb_test_result: None,
            sqlserver_form: SQLServerConnectionForm::new(theme),
            sqlserver_form_data: SQLServerConnectionFormData::new(),
            sqlserver_testing: false,
            sqlserver_test_result: None,
            oracle_form: OracleConnectionForm::new(theme),
            oracle_form_data: OracleConnectionFormData::new(),
            oracle_testing: false,
            oracle_test_result: None,
            // Connection management
            connection_manager: ConnectionManager::new(),
            pending_connection_config: None,
            // Query execution
            executing_query: false,
            query_results: HashMap::new(),
            query_errors: HashMap::new(),
        }
    }
}

/// Result of a connection operation
#[derive(Debug, Clone)]
enum ConnectionOperation {
    Connected(String), // connection name
    Disconnected,
}

#[derive(Debug, Clone)]
enum Message {
    MenuToggle(MenuItem),
    MenuAction(MenuAction),
    CloseMenu,
    LeftPanelTabClicked(ComponentId),
    ResizeStart,
    ResizeMove(f32),
    ResizeEnd,
    NewMainTab,
    MainTabClicked(TabId),
    MainTabClosed(TabId),
    QueryEditor(TabId, QueryEditorMessage),
    QueryExecutionResult(TabId, Result<QueryResult, String>),
    NewConnection,
    ConnectToDatabase(ConnectionConfig, Option<String>), // config, password
    DisconnectFromDatabase(String),                      // connection ID
    ReconnectToDatabase(String),                         // connection ID
    ConnectionOperationResult(String, Result<ConnectionOperation, String>), // connection ID, operation result
    ConnectionForm(ConnectionFormMessage),
    ConnectionTestResult(Result<(), String>),
    ComponentAction(ComponentAction),
    ShowSettings,
    SettingsEditor(SettingsEditorMessage),
    // Adapter-specific connection forms
    AdapterSelector(AdapterSelectorMessage),
    PostgresForm(PostgresConnectionFormMessage),
    PostgresTestResult(Result<(), String>),
    MySQLForm(MySQLConnectionFormMessage),
    MySQLTestResult(Result<(), String>),
    SQLiteForm(SQLiteConnectionFormMessage),
    SQLiteTestResult(Result<(), String>),
    MongoDBForm(MongoDBConnectionFormMessage),
    MongoDBTestResult(Result<(), String>),
    SQLServerForm(SQLServerConnectionFormMessage),
    SQLServerTestResult(Result<(), String>),
    OracleForm(OracleConnectionFormMessage),
    OracleTestResult(Result<(), String>),
}

impl From<ComponentAction> for Message {
    fn from(action: ComponentAction) -> Self {
        Message::ComponentAction(action)
    }
}

/// Test database connection for adapter-specific forms
async fn test_connection_from_form(
    config: ConnectionConfig,
    password: Option<String>,
) -> Result<(), String> {
    use std::time::Duration;
    use tokio::time::timeout;

    let result = timeout(
        Duration::from_secs(10),
        test_database_connection(config, password),
    )
    .await;

    match result {
        Ok(Ok(success)) => {
            if success {
                Ok(())
            } else {
                Err("Connection failed: Unable to connect to database".to_string())
            }
        }
        Ok(Err(e)) => Err(format!("Connection error: {}", e)),
        Err(_) => Err("Connection timeout: Failed to connect within 10 seconds".to_string()),
    }
}

/// Test database connection with appropriate adapter
async fn test_database_connection(
    config: ConnectionConfig,
    password: Option<String>,
) -> Result<bool, String> {
    let mut conn = rusty_app::connection_manager::ActiveConnection::new(config, password);
    conn.connect().await.map(|_| true)
}

impl DatabaseIDE {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MenuToggle(menu) => {
                // Toggle menu open/close
                if self.open_menu == Some(menu) {
                    self.open_menu = None;
                } else {
                    self.open_menu = Some(menu);
                }
                Task::none()
            }
            Message::MenuAction(action) => {
                // Close menu after action
                self.open_menu = None;

                // Handle menu actions
                use rusty_app::menu_bar::{FileMenuItem, ViewMenuItem};
                match action {
                    MenuAction::File(FileMenuItem::NewConnection) => {
                        self.showing_connection_form = true;
                        self.connection_form_data = ConnectionFormData::new();
                    }
                    MenuAction::File(FileMenuItem::Exit) => {
                        // TODO: Implement clean exit
                        println!("Exit requested");
                    }
                    MenuAction::View(view_item) => {
                        match view_item {
                            ViewMenuItem::ToggleLeftPanel => {
                                self.show_left_panel = !self.show_left_panel;
                            }
                            ViewMenuItem::Settings => {
                                self.showing_settings_editor = true;
                                self.settings_editor_data =
                                    SettingsEditorData::from_manager(&self.settings_manager);
                                self.settings_validation_error = None;
                                self.settings_success_message = None;
                            }
                            _ => {
                                // Toggle component views in ViewRegistry
                                if let Some(component_id) = view_item.as_component_id() {
                                    self.view_registry.toggle(component_id);
                                }
                            }
                        }
                    }
                    _ => {
                        // Log other menu actions (temporary implementation)
                        println!("Menu action: {:?}", action);
                    }
                }
                Task::none()
            }
            Message::CloseMenu => {
                self.open_menu = None;
                Task::none()
            }
            Message::LeftPanelTabClicked(component_id) => {
                self.active_component = Some(component_id);
                Task::none()
            }
            Message::ResizeStart => {
                self.is_resizing = true;
                self.last_mouse_x = None;
                Task::none()
            }
            Message::ResizeMove(mouse_x) => {
                if self.is_resizing {
                    if let Some(last_x) = self.last_mouse_x {
                        let delta_x = mouse_x - last_x;
                        self.panel_width = left_panel::constrain_width(self.panel_width + delta_x);
                    }
                    self.last_mouse_x = Some(mouse_x);
                }
                Task::none()
            }
            Message::ResizeEnd => {
                self.is_resizing = false;
                self.last_mouse_x = None;
                Task::none()
            }
            Message::NewMainTab => {
                let tab_count = self.main_panel.tabs().len() + 1;
                self.main_panel.add_tab(format!("Query {}", tab_count));
                Task::none()
            }
            Message::MainTabClicked(id) => {
                self.main_panel.set_active_tab(id);
                Task::none()
            }
            Message::MainTabClosed(id) => {
                self.main_panel.close_tab(id);
                Task::none()
            }
            Message::QueryEditor(tab_id, query_msg) => {
                // Handle query editor messages
                match query_msg {
                    QueryEditorMessage::ActionPerformed(_action) => {
                        // Text changed - no action needed
                        Task::none()
                    }
                    QueryEditorMessage::Execute => {
                        // Execute query
                        self.execute_query(tab_id)
                    }
                }
            }
            Message::QueryExecutionResult(tab_id, result) => {
                self.executing_query = false;

                match result {
                    Ok(query_result) => {
                        // Store successful result
                        self.query_results.insert(tab_id, Some(query_result));
                        self.query_errors.insert(tab_id, None);
                        info!("Query executed successfully for tab {}", tab_id);
                    }
                    Err(error) => {
                        // Store error
                        self.query_results.insert(tab_id, None);
                        self.query_errors.insert(tab_id, Some(error.clone()));
                        warn!("Query execution failed for tab {}: {}", tab_id, error);
                    }
                }

                Task::none()
            }
            Message::NewConnection => {
                // Show adapter selector instead of going directly to form
                self.showing_adapter_selector = true;
                self.selected_adapter = None;
                Task::none()
            }
            Message::ConnectionForm(form_message) => {
                match form_message {
                    ConnectionFormMessage::Save => {
                        // Validate form data
                        if let Err(e) = self.connection_form_data.validate() {
                            // TODO: Show error message to user
                            println!("Validation error: {}", e);
                            return Task::none();
                        }

                        // Convert form data to ConnectionConfig
                        let config = self.form_data_to_config(&self.connection_form_data);

                        // Add to saved connections
                        self.saved_connections.push(config);

                        // Save to settings.toml
                        self.settings_manager.settings_mut().connections =
                            self.saved_connections.clone();
                        if let Err(e) = self.settings_manager.save() {
                            // TODO: Show error message to user
                            println!("Failed to save connection: {}", e);
                        } else {
                            // Close the form on success
                            self.showing_connection_form = false;
                        }

                        Task::none()
                    }
                    ConnectionFormMessage::Cancel => {
                        self.showing_connection_form = false;
                        Task::none()
                    }
                    ConnectionFormMessage::NameChanged(name) => {
                        self.connection_form_data.name = name;
                        Task::none()
                    }
                    ConnectionFormMessage::DbTypeChanged(db_type) => {
                        self.connection_form_data =
                            self.connection_form_data.clone().with_db_type(db_type);
                        Task::none()
                    }
                    ConnectionFormMessage::HostChanged(host) => {
                        self.connection_form_data.host = host;
                        Task::none()
                    }
                    ConnectionFormMessage::PortChanged(port) => {
                        self.connection_form_data.port = port;
                        Task::none()
                    }
                    ConnectionFormMessage::DatabaseChanged(database) => {
                        self.connection_form_data.database = database;
                        Task::none()
                    }
                    ConnectionFormMessage::UsernameChanged(username) => {
                        self.connection_form_data.username = username;
                        Task::none()
                    }
                    ConnectionFormMessage::PasswordChanged(password) => {
                        self.connection_form_data.password = password;
                        Task::none()
                    }
                    ConnectionFormMessage::FilePathChanged(file_path) => {
                        self.connection_form_data.file_path = file_path;
                        Task::none()
                    }
                    ConnectionFormMessage::TestConnection => {
                        // Validate form data first
                        if let Err(e) = self.connection_form_data.validate() {
                            // Show error - for now just set test result
                            self.test_result = Some(Err(e));
                            return Task::none();
                        }

                        // Set testing state
                        self.testing_connection = true;
                        self.test_result = None;

                        // Create async task to test connection
                        self.test_connection_async()
                    }
                }
            }
            Message::ConnectionTestResult(result) => {
                self.testing_connection = false;
                self.test_result = Some(result);
                Task::none()
            }
            Message::ComponentAction(action) => {
                // Placeholder: handle component actions
                // TODO: Implement specific handlers for each component action variant
                println!("Component action: {:?}", action);
                Task::none()
            }
            Message::ShowSettings => {
                self.showing_settings_editor = true;
                self.settings_editor_data =
                    SettingsEditorData::from_manager(&self.settings_manager);
                self.settings_validation_error = None;
                self.settings_success_message = None;
                Task::none()
            }
            Message::SettingsEditor(editor_message) => {
                match editor_message {
                    SettingsEditorMessage::Cancel => {
                        self.showing_settings_editor = false;
                        Task::none()
                    }
                    // Logging settings
                    SettingsEditorMessage::FilterChanged(value) => {
                        self.settings_editor_data.logging.filter = value;
                        Task::none()
                    }
                    SettingsEditorMessage::ConsoleFilterChanged(value) => {
                        self.settings_editor_data.logging.console_filter =
                            if value.is_empty() { None } else { Some(value) };
                        Task::none()
                    }
                    SettingsEditorMessage::FileFilterChanged(value) => {
                        self.settings_editor_data.logging.file_filter =
                            if value.is_empty() { None } else { Some(value) };
                        Task::none()
                    }
                    SettingsEditorMessage::ConsoleFormatChanged(format) => {
                        self.settings_editor_data.logging.console_format = format;
                        Task::none()
                    }
                    SettingsEditorMessage::ConsoleWriterChanged(writer) => {
                        self.settings_editor_data.logging.console_writer = writer;
                        Task::none()
                    }
                    SettingsEditorMessage::ConsoleEnabledToggled => {
                        self.settings_editor_data.logging.console_enabled =
                            !self.settings_editor_data.logging.console_enabled;
                        Task::none()
                    }
                    SettingsEditorMessage::FileFormatChanged(format) => {
                        self.settings_editor_data.logging.file_format = format;
                        Task::none()
                    }
                    SettingsEditorMessage::FileEnabledToggled => {
                        self.settings_editor_data.logging.file_enabled =
                            !self.settings_editor_data.logging.file_enabled;
                        Task::none()
                    }
                    SettingsEditorMessage::FileDirectoryChanged(value) => {
                        self.settings_editor_data.logging.file_directory = value;
                        Task::none()
                    }
                    SettingsEditorMessage::FilePrefixChanged(value) => {
                        self.settings_editor_data.logging.file_prefix = value;
                        Task::none()
                    }
                    SettingsEditorMessage::RotationPolicyChanged(policy) => {
                        self.settings_editor_data.logging.rotation_policy = policy;
                        Task::none()
                    }
                    // UI preferences
                    SettingsEditorMessage::ThemeChanged(value) => {
                        self.settings_editor_data.ui_preferences.theme = value;
                        Task::none()
                    }
                    SettingsEditorMessage::PanelWidthChanged(value) => {
                        // Parse string to u32, keep current value if parse fails
                        if let Ok(width) = value.parse::<u32>() {
                            self.settings_editor_data.ui_preferences.panel_width = width;
                        }
                        Task::none()
                    }
                    SettingsEditorMessage::ShowLeftPanelToggled => {
                        self.settings_editor_data.ui_preferences.show_left_panel =
                            !self.settings_editor_data.ui_preferences.show_left_panel;
                        Task::none()
                    }
                    SettingsEditorMessage::Save => {
                        // Update settings_manager with edited values
                        self.settings_manager.settings_mut().logging =
                            self.settings_editor_data.logging.clone();
                        self.settings_manager.settings_mut().ui_preferences =
                            self.settings_editor_data.ui_preferences.clone();

                        // Validate settings
                        if let Err(e) = self.settings_manager.validate() {
                            self.settings_validation_error = Some(e.to_string());
                            self.settings_success_message = None;
                            return Task::none();
                        }

                        // Save settings to disk
                        if let Err(e) = self.settings_manager.save() {
                            self.settings_validation_error = Some(format!("Failed to save: {}", e));
                            self.settings_success_message = None;
                            return Task::none();
                        }

                        // Rebuild and apply logging configuration
                        if let Ok(logging_config) =
                            build_logging_config(&self.settings_manager.settings().logging)
                        {
                            if let Err(e) = logging_config.apply() {
                                self.settings_validation_error =
                                    Some(format!("Failed to reload logging: {}", e));
                                self.settings_success_message = None;
                                return Task::none();
                            }
                        } else {
                            self.settings_validation_error =
                                Some("Invalid logging configuration".to_string());
                            self.settings_success_message = None;
                            return Task::none();
                        }

                        // Update DatabaseIDE UI fields from new settings
                        let ui_prefs = &self.settings_manager.settings().ui_preferences;
                        self.panel_width = ui_prefs.panel_width as f32;
                        self.show_left_panel = ui_prefs.show_left_panel;

                        // Update theme if changed
                        let new_theme = if ui_prefs.theme == "dark" {
                            ThemeColors::dark()
                        } else {
                            ThemeColors::dark() // TODO: Add ThemeColors::light()
                        };
                        self.theme = new_theme;

                        // Update all components with new theme
                        self.menu_bar = MenuBar::new(new_theme);
                        self.left_panel = LeftPanel::new(new_theme);
                        self.status_bar = StatusBar::new(new_theme);
                        self.connection_form = ConnectionForm::new(new_theme);
                        self.settings_editor = SettingsEditor::new(new_theme);

                        // Show success and close editor
                        self.settings_success_message =
                            Some("Settings saved successfully".to_string());
                        self.settings_validation_error = None;
                        self.showing_settings_editor = false;

                        Task::none()
                    }
                    SettingsEditorMessage::Reload => {
                        // Reload settings from disk
                        if let Err(e) = self.settings_manager.reload() {
                            self.settings_validation_error =
                                Some(format!("Failed to reload: {}", e));
                            self.settings_success_message = None;
                            return Task::none();
                        }

                        // Reset editor data from reloaded settings
                        self.settings_editor_data =
                            SettingsEditorData::from_manager(&self.settings_manager);

                        // Clear errors and show success
                        self.settings_validation_error = None;
                        self.settings_success_message =
                            Some("Settings reloaded from disk".to_string());

                        // Keep settings editor open (don't close)
                        Task::none()
                    }
                }
            }
            // Adapter selector messages
            Message::AdapterSelector(selector_message) => {
                match selector_message {
                    AdapterSelectorMessage::AdapterSelected(db_type) => {
                        self.selected_adapter = Some(db_type);
                        Task::none()
                    }
                    AdapterSelectorMessage::Continue => {
                        // Hide adapter selector and show the appropriate form
                        self.showing_adapter_selector = false;
                        if let Some(adapter) = self.selected_adapter {
                            self.showing_connection_form = true;
                            match adapter {
                                DatabaseType::Postgres => {
                                    self.postgres_form_data = PostgresConnectionFormData::new();
                                    self.postgres_test_result = None;
                                }
                                DatabaseType::MySQL => {
                                    self.mysql_form_data = MySQLConnectionFormData::new();
                                    self.mysql_test_result = None;
                                }
                                DatabaseType::SQLite => {
                                    self.sqlite_form_data = SQLiteConnectionFormData::new();
                                    self.sqlite_test_result = None;
                                }
                                DatabaseType::MongoDB => {
                                    self.mongodb_form_data = MongoDBConnectionFormData::new();
                                    self.mongodb_test_result = None;
                                }
                                DatabaseType::SQLServer => {
                                    self.sqlserver_form_data = SQLServerConnectionFormData::new();
                                    self.sqlserver_test_result = None;
                                }
                                DatabaseType::Oracle => {
                                    self.oracle_form_data = OracleConnectionFormData::new();
                                    self.oracle_test_result = None;
                                }
                                DatabaseType::DuckDB => unreachable!("DuckDB not used in rusty-app"),
                            }
                        }
                        Task::none()
                    }
                    AdapterSelectorMessage::Cancel => {
                        self.showing_adapter_selector = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                }
            }
            // PostgreSQL form messages
            Message::PostgresForm(form_message) => {
                match form_message {
                    PostgresConnectionFormMessage::Cancel => {
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                    PostgresConnectionFormMessage::NameChanged(name) => {
                        self.postgres_form_data.name = name;
                        self.postgres_test_result = None;
                        Task::none()
                    }
                    PostgresConnectionFormMessage::HostChanged(host) => {
                        self.postgres_form_data.host = host;
                        self.postgres_test_result = None;
                        Task::none()
                    }
                    PostgresConnectionFormMessage::PortChanged(port) => {
                        self.postgres_form_data.port = port;
                        self.postgres_test_result = None;
                        Task::none()
                    }
                    PostgresConnectionFormMessage::DatabaseChanged(database) => {
                        self.postgres_form_data.database = database;
                        self.postgres_test_result = None;
                        Task::none()
                    }
                    PostgresConnectionFormMessage::UsernameChanged(username) => {
                        self.postgres_form_data.username = username;
                        self.postgres_test_result = None;
                        Task::none()
                    }
                    PostgresConnectionFormMessage::PasswordChanged(password) => {
                        self.postgres_form_data.password = password;
                        self.postgres_test_result = None;
                        Task::none()
                    }
                    PostgresConnectionFormMessage::TestConnection => {
                        self.postgres_testing = true;
                        self.postgres_test_result = None;
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.postgres_form_data.name.clone(),
                            db_type: DatabaseType::Postgres,
                            host: Some(self.postgres_form_data.host.clone()),
                            port: self.postgres_form_data.port.parse().ok(),
                            database: self.postgres_form_data.database.clone(),
                            username: Some(self.postgres_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };
                        let password = if self.postgres_form_data.password.is_empty() {
                            None
                        } else {
                            Some(self.postgres_form_data.password.clone())
                        };
                        Task::perform(
                            test_connection_from_form(config, password),
                            Message::PostgresTestResult,
                        )
                    }
                    PostgresConnectionFormMessage::Save => {
                        // Validate form
                        if let Err(e) = self.postgres_form_data.validate() {
                            self.postgres_test_result = Some(Err(e));
                            return Task::none();
                        }

                        // Create connection config
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.postgres_form_data.name.clone(),
                            db_type: DatabaseType::Postgres,
                            host: Some(self.postgres_form_data.host.clone()),
                            port: self.postgres_form_data.port.parse().ok(),
                            database: self.postgres_form_data.database.clone(),
                            username: Some(self.postgres_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };

                        // Save to settings
                        self.settings_manager
                            .settings_mut()
                            .connections
                            .push(config.clone());
                        if let Err(e) = self.settings_manager.save() {
                            warn!("Failed to save connection: {}", e);
                            self.postgres_test_result = Some(Err(format!("Failed to save: {}", e)));
                            return Task::none();
                        }

                        // Update saved_connections
                        self.saved_connections.push(config);

                        // Close form
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                }
            }
            Message::PostgresTestResult(result) => {
                self.postgres_testing = false;
                self.postgres_test_result = Some(result.map(|_| ()));
                Task::none()
            }
            // MySQL form messages
            Message::MySQLForm(form_message) => {
                match form_message {
                    MySQLConnectionFormMessage::Cancel => {
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                    MySQLConnectionFormMessage::NameChanged(name) => {
                        self.mysql_form_data.name = name;
                        self.mysql_test_result = None;
                        Task::none()
                    }
                    MySQLConnectionFormMessage::HostChanged(host) => {
                        self.mysql_form_data.host = host;
                        self.mysql_test_result = None;
                        Task::none()
                    }
                    MySQLConnectionFormMessage::PortChanged(port) => {
                        self.mysql_form_data.port = port;
                        self.mysql_test_result = None;
                        Task::none()
                    }
                    MySQLConnectionFormMessage::DatabaseChanged(database) => {
                        self.mysql_form_data.database = database;
                        self.mysql_test_result = None;
                        Task::none()
                    }
                    MySQLConnectionFormMessage::UsernameChanged(username) => {
                        self.mysql_form_data.username = username;
                        self.mysql_test_result = None;
                        Task::none()
                    }
                    MySQLConnectionFormMessage::PasswordChanged(password) => {
                        self.mysql_form_data.password = password;
                        self.mysql_test_result = None;
                        Task::none()
                    }
                    MySQLConnectionFormMessage::TestConnection => {
                        self.mysql_testing = true;
                        self.mysql_test_result = None;
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.mysql_form_data.name.clone(),
                            db_type: DatabaseType::MySQL,
                            host: Some(self.mysql_form_data.host.clone()),
                            port: self.mysql_form_data.port.parse().ok(),
                            database: self.mysql_form_data.database.clone(),
                            username: Some(self.mysql_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };
                        let password = if self.mysql_form_data.password.is_empty() {
                            None
                        } else {
                            Some(self.mysql_form_data.password.clone())
                        };
                        Task::perform(
                            test_connection_from_form(config, password),
                            Message::MySQLTestResult,
                        )
                    }
                    MySQLConnectionFormMessage::Save => {
                        // Validate form
                        if let Err(e) = self.mysql_form_data.validate() {
                            self.mysql_test_result = Some(Err(e));
                            return Task::none();
                        }

                        // Create connection config
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.mysql_form_data.name.clone(),
                            db_type: DatabaseType::MySQL,
                            host: Some(self.mysql_form_data.host.clone()),
                            port: self.mysql_form_data.port.parse().ok(),
                            database: self.mysql_form_data.database.clone(),
                            username: Some(self.mysql_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };

                        // Save to settings
                        self.settings_manager
                            .settings_mut()
                            .connections
                            .push(config.clone());
                        if let Err(e) = self.settings_manager.save() {
                            warn!("Failed to save connection: {}", e);
                            self.mysql_test_result = Some(Err(format!("Failed to save: {}", e)));
                            return Task::none();
                        }

                        // Update saved_connections
                        self.saved_connections.push(config);

                        // Close form
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                }
            }
            Message::MySQLTestResult(result) => {
                self.mysql_testing = false;
                self.mysql_test_result = Some(result.map(|_| ()));
                Task::none()
            }
            // SQLite form messages
            Message::SQLiteForm(form_message) => {
                match form_message {
                    SQLiteConnectionFormMessage::Cancel => {
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                    SQLiteConnectionFormMessage::NameChanged(name) => {
                        self.sqlite_form_data.name = name;
                        self.sqlite_test_result = None;
                        Task::none()
                    }
                    SQLiteConnectionFormMessage::FilePathChanged(path) => {
                        self.sqlite_form_data.file_path = path;
                        self.sqlite_test_result = None;
                        Task::none()
                    }
                    SQLiteConnectionFormMessage::TestConnection => {
                        self.sqlite_testing = true;
                        self.sqlite_test_result = None;
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.sqlite_form_data.name.clone(),
                            db_type: DatabaseType::SQLite,
                            host: None,
                            port: None,
                            database: self.sqlite_form_data.file_path.clone(),
                            username: None,
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };
                        Task::perform(
                            test_connection_from_form(config, None),
                            Message::SQLiteTestResult,
                        )
                    }
                    SQLiteConnectionFormMessage::Save => {
                        // Validate form
                        if let Err(e) = self.sqlite_form_data.validate() {
                            self.sqlite_test_result = Some(Err(e));
                            return Task::none();
                        }

                        // Create connection config
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.sqlite_form_data.name.clone(),
                            db_type: DatabaseType::SQLite,
                            host: None,
                            port: None,
                            database: self.sqlite_form_data.file_path.clone(),
                            username: None,
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };

                        // Save to settings
                        self.settings_manager
                            .settings_mut()
                            .connections
                            .push(config.clone());
                        if let Err(e) = self.settings_manager.save() {
                            warn!("Failed to save connection: {}", e);
                            self.sqlite_test_result = Some(Err(format!("Failed to save: {}", e)));
                            return Task::none();
                        }

                        // Update saved_connections
                        self.saved_connections.push(config);

                        // Close form
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                }
            }
            Message::SQLiteTestResult(result) => {
                self.sqlite_testing = false;
                self.sqlite_test_result = Some(result.map(|_| ()));
                Task::none()
            }
            // MongoDB form messages
            Message::MongoDBForm(form_message) => {
                match form_message {
                    MongoDBConnectionFormMessage::Cancel => {
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                    MongoDBConnectionFormMessage::NameChanged(name) => {
                        self.mongodb_form_data.name = name;
                        self.mongodb_test_result = None;
                        Task::none()
                    }
                    MongoDBConnectionFormMessage::HostChanged(host) => {
                        self.mongodb_form_data.host = host;
                        self.mongodb_test_result = None;
                        Task::none()
                    }
                    MongoDBConnectionFormMessage::PortChanged(port) => {
                        self.mongodb_form_data.port = port;
                        self.mongodb_test_result = None;
                        Task::none()
                    }
                    MongoDBConnectionFormMessage::DatabaseChanged(database) => {
                        self.mongodb_form_data.database = database;
                        self.mongodb_test_result = None;
                        Task::none()
                    }
                    MongoDBConnectionFormMessage::UsernameChanged(username) => {
                        self.mongodb_form_data.username = username;
                        self.mongodb_test_result = None;
                        Task::none()
                    }
                    MongoDBConnectionFormMessage::PasswordChanged(password) => {
                        self.mongodb_form_data.password = password;
                        self.mongodb_test_result = None;
                        Task::none()
                    }
                    MongoDBConnectionFormMessage::TestConnection => {
                        self.mongodb_testing = true;
                        self.mongodb_test_result = None;
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.mongodb_form_data.name.clone(),
                            db_type: DatabaseType::MongoDB,
                            host: Some(self.mongodb_form_data.host.clone()),
                            port: self.mongodb_form_data.port.parse().ok(),
                            database: self.mongodb_form_data.database.clone(),
                            username: Some(self.mongodb_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };
                        let password = if self.mongodb_form_data.password.is_empty() {
                            None
                        } else {
                            Some(self.mongodb_form_data.password.clone())
                        };
                        Task::perform(
                            test_connection_from_form(config, password),
                            Message::MongoDBTestResult,
                        )
                    }
                    MongoDBConnectionFormMessage::Save => {
                        // Validate form
                        if let Err(e) = self.mongodb_form_data.validate() {
                            self.mongodb_test_result = Some(Err(e));
                            return Task::none();
                        }

                        // Create connection config
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.mongodb_form_data.name.clone(),
                            db_type: DatabaseType::MongoDB,
                            host: Some(self.mongodb_form_data.host.clone()),
                            port: self.mongodb_form_data.port.parse().ok(),
                            database: self.mongodb_form_data.database.clone(),
                            username: Some(self.mongodb_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };

                        // Save to settings
                        self.settings_manager
                            .settings_mut()
                            .connections
                            .push(config.clone());
                        if let Err(e) = self.settings_manager.save() {
                            warn!("Failed to save connection: {}", e);
                            self.mongodb_test_result = Some(Err(format!("Failed to save: {}", e)));
                            return Task::none();
                        }

                        // Update saved_connections
                        self.saved_connections.push(config);

                        // Close form
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                }
            }
            Message::MongoDBTestResult(result) => {
                self.mongodb_testing = false;
                self.mongodb_test_result = Some(result.map(|_| ()));
                Task::none()
            }
            // SQL Server form messages
            Message::SQLServerForm(form_message) => {
                match form_message {
                    SQLServerConnectionFormMessage::Cancel => {
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                    SQLServerConnectionFormMessage::NameChanged(name) => {
                        self.sqlserver_form_data.name = name;
                        self.sqlserver_test_result = None;
                        Task::none()
                    }
                    SQLServerConnectionFormMessage::HostChanged(host) => {
                        self.sqlserver_form_data.host = host;
                        self.sqlserver_test_result = None;
                        Task::none()
                    }
                    SQLServerConnectionFormMessage::PortChanged(port) => {
                        self.sqlserver_form_data.port = port;
                        self.sqlserver_test_result = None;
                        Task::none()
                    }
                    SQLServerConnectionFormMessage::DatabaseChanged(database) => {
                        self.sqlserver_form_data.database = database;
                        self.sqlserver_test_result = None;
                        Task::none()
                    }
                    SQLServerConnectionFormMessage::UsernameChanged(username) => {
                        self.sqlserver_form_data.username = username;
                        self.sqlserver_test_result = None;
                        Task::none()
                    }
                    SQLServerConnectionFormMessage::PasswordChanged(password) => {
                        self.sqlserver_form_data.password = password;
                        self.sqlserver_test_result = None;
                        Task::none()
                    }
                    SQLServerConnectionFormMessage::TestConnection => {
                        self.sqlserver_testing = true;
                        self.sqlserver_test_result = None;
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.sqlserver_form_data.name.clone(),
                            db_type: DatabaseType::SQLServer,
                            host: Some(self.sqlserver_form_data.host.clone()),
                            port: self.sqlserver_form_data.port.parse().ok(),
                            database: self.sqlserver_form_data.database.clone(),
                            username: Some(self.sqlserver_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };
                        let password = if self.sqlserver_form_data.password.is_empty() {
                            None
                        } else {
                            Some(self.sqlserver_form_data.password.clone())
                        };
                        Task::perform(
                            test_connection_from_form(config, password),
                            Message::SQLServerTestResult,
                        )
                    }
                    SQLServerConnectionFormMessage::Save => {
                        // Validate form
                        if let Err(e) = self.sqlserver_form_data.validate() {
                            self.sqlserver_test_result = Some(Err(e));
                            return Task::none();
                        }

                        // Create connection config
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.sqlserver_form_data.name.clone(),
                            db_type: DatabaseType::SQLServer,
                            host: Some(self.sqlserver_form_data.host.clone()),
                            port: self.sqlserver_form_data.port.parse().ok(),
                            database: self.sqlserver_form_data.database.clone(),
                            username: Some(self.sqlserver_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };

                        // Save to settings
                        self.settings_manager
                            .settings_mut()
                            .connections
                            .push(config.clone());
                        if let Err(e) = self.settings_manager.save() {
                            warn!("Failed to save connection: {}", e);
                            self.sqlserver_test_result =
                                Some(Err(format!("Failed to save: {}", e)));
                            return Task::none();
                        }

                        // Update saved_connections
                        self.saved_connections.push(config);

                        // Close form
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                }
            }
            Message::SQLServerTestResult(result) => {
                self.sqlserver_testing = false;
                self.sqlserver_test_result = Some(result.map(|_| ()));
                Task::none()
            }
            // Oracle form messages
            Message::OracleForm(form_message) => {
                match form_message {
                    OracleConnectionFormMessage::Cancel => {
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                    OracleConnectionFormMessage::NameChanged(name) => {
                        self.oracle_form_data.name = name;
                        self.oracle_test_result = None;
                        Task::none()
                    }
                    OracleConnectionFormMessage::HostChanged(host) => {
                        self.oracle_form_data.host = host;
                        self.oracle_test_result = None;
                        Task::none()
                    }
                    OracleConnectionFormMessage::PortChanged(port) => {
                        self.oracle_form_data.port = port;
                        self.oracle_test_result = None;
                        Task::none()
                    }
                    OracleConnectionFormMessage::DatabaseChanged(database) => {
                        self.oracle_form_data.database = database;
                        self.oracle_test_result = None;
                        Task::none()
                    }
                    OracleConnectionFormMessage::UsernameChanged(username) => {
                        self.oracle_form_data.username = username;
                        self.oracle_test_result = None;
                        Task::none()
                    }
                    OracleConnectionFormMessage::PasswordChanged(password) => {
                        self.oracle_form_data.password = password;
                        self.oracle_test_result = None;
                        Task::none()
                    }
                    OracleConnectionFormMessage::TestConnection => {
                        self.oracle_testing = true;
                        self.oracle_test_result = None;
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.oracle_form_data.name.clone(),
                            db_type: DatabaseType::Oracle,
                            host: Some(self.oracle_form_data.host.clone()),
                            port: self.oracle_form_data.port.parse().ok(),
                            database: self.oracle_form_data.database.clone(),
                            username: Some(self.oracle_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };
                        let password = if self.oracle_form_data.password.is_empty() {
                            None
                        } else {
                            Some(self.oracle_form_data.password.clone())
                        };
                        Task::perform(
                            test_connection_from_form(config, password),
                            Message::OracleTestResult,
                        )
                    }
                    OracleConnectionFormMessage::Save => {
                        // Validate form
                        if let Err(e) = self.oracle_form_data.validate() {
                            self.oracle_test_result = Some(Err(e));
                            return Task::none();
                        }

                        // Create connection config
                        let config = ConnectionConfig {
                            id: format!(
                                "conn-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                            ),
                            name: self.oracle_form_data.name.clone(),
                            db_type: DatabaseType::Oracle,
                            host: Some(self.oracle_form_data.host.clone()),
                            port: self.oracle_form_data.port.parse().ok(),
                            database: self.oracle_form_data.database.clone(),
                            username: Some(self.oracle_form_data.username.clone()),
                            use_ssl: false,
                            parameters: HashMap::new(),
                            pool_config: None,
                        };

                        // Save to settings
                        self.settings_manager
                            .settings_mut()
                            .connections
                            .push(config.clone());
                        if let Err(e) = self.settings_manager.save() {
                            warn!("Failed to save connection: {}", e);
                            self.oracle_test_result = Some(Err(format!("Failed to save: {}", e)));
                            return Task::none();
                        }

                        // Update saved_connections
                        self.saved_connections.push(config);

                        // Close form
                        self.showing_connection_form = false;
                        self.selected_adapter = None;
                        Task::none()
                    }
                }
            }
            Message::OracleTestResult(result) => {
                self.oracle_testing = false;
                self.oracle_test_result = Some(result.map(|_| ()));
                Task::none()
            }
            Message::ConnectToDatabase(config, password) => {
                // Set status to connecting
                self.connection_status = ConnectionStatus::Connecting(config.name.clone());

                // Store config for later use
                self.pending_connection_config = Some((config.clone(), password.clone()));

                // Create active connection and connect it
                let config_clone = config.clone();
                let password_clone = password.clone();

                Task::perform(
                    async move {
                        let mut connection =
                            ActiveConnection::new(config_clone.clone(), password_clone.clone());
                        let id = connection.id.clone();
                        let name = config_clone.name.clone();
                        match connection.connect().await {
                            Ok(()) => Ok((id, name)),
                            Err(e) => Err((id, e)),
                        }
                    },
                    |result| match result {
                        Ok((id, name)) => Message::ConnectionOperationResult(
                            id,
                            Ok(ConnectionOperation::Connected(name)),
                        ),
                        Err((id, e)) => Message::ConnectionOperationResult(id, Err(e)),
                    },
                )
            }
            Message::DisconnectFromDatabase(connection_id) => {
                if self
                    .connection_manager
                    .get_connection(&connection_id)
                    .is_some()
                {
                    Task::perform(
                        async move {
                            // Simulate disconnect operation - return the ID with result (no connection to store)
                            Ok((connection_id, ()))
                        },
                        |result| match result {
                            Ok((id, ())) => Message::ConnectionOperationResult(
                                id,
                                Ok(ConnectionOperation::Disconnected),
                            ),
                            Err((id, e)) => Message::ConnectionOperationResult(id, Err(e)),
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::ReconnectToDatabase(connection_id) => {
                if let Some(connection) = self.connection_manager.get_connection(&connection_id) {
                    let config = connection.config.clone();
                    let password = connection.password.clone();
                    self.update(Message::ConnectToDatabase(config, password))
                } else {
                    Task::none()
                }
            }
            Message::ConnectionOperationResult(connection_id, result) => {
                match result {
                    Ok(ConnectionOperation::Connected(name)) => {
                        // Connection successful - create and store the connection
                        if let Some((config, password)) = self.pending_connection_config.take() {
                            if config.id == connection_id {
                                // Create a new connected connection and add to manager
                                let mut connection = ActiveConnection::new(config, password);
                                // Mark as connected (connect was already successful in async task)
                                connection.status =
                                    rusty_app::connection_manager::ConnectionStatus::Connected;
                                self.connection_manager.add_connection(connection);
                                self.connection_manager.set_selected(Some(connection_id));
                                self.connection_status = ConnectionStatus::Connected(name);
                                return Task::none();
                            }
                        }
                        // No pending config - just update status
                        self.connection_status = ConnectionStatus::Connected(name);
                    }
                    Ok(ConnectionOperation::Disconnected) => {
                        // Disconnect successful - update status
                        self.connection_status = ConnectionStatus::Disconnected;
                    }
                    Err(error) => {
                        // Operation failed - show error and clear pending
                        self.pending_connection_config = None;
                        self.connection_status = ConnectionStatus::Error(error);
                    }
                }
                Task::none()
            }
        }
    }

    /// Subscribe to mouse events for drag-to-resize functionality
    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _status, _window| match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Message::ResizeMove(position.x))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(Message::ResizeEnd)
            }
            _ => None,
        })
    }

    /// Convert ConnectionFormData to ConnectionConfig
    fn form_data_to_config(&self, form_data: &ConnectionFormData) -> ConnectionConfig {

        // Generate a unique ID from the connection name
        let id = form_data.name.to_lowercase().replace(' ', "-");

        // For SQLite, database field holds the file path
        let (host, port, database) = match form_data.db_type {
            DatabaseType::SQLite => (None, None, form_data.file_path.clone()),
            _ => {
                let port = form_data.port.parse::<u16>().ok();
                (
                    Some(form_data.host.clone()),
                    port,
                    form_data.database.clone(),
                )
            }
        };

        let username = if form_data.username.is_empty() {
            None
        } else {
            Some(form_data.username.clone())
        };

        ConnectionConfig {
            id,
            name: form_data.name.clone(),
            db_type: form_data.db_type,
            host,
            port,
            database,
            username,
            use_ssl: false, // TODO: Add SSL checkbox to form
            parameters: HashMap::new(),
            pool_config: None,
        }
    }

    /// Test connection asynchronously with 10 second timeout
    fn test_connection_async(&self) -> Task<Message> {
        let config = self.form_data_to_config(&self.connection_form_data);
        let password = if self.connection_form_data.password.is_empty() {
            None
        } else {
            Some(self.connection_form_data.password.clone())
        };

        Task::perform(
            async move {
                // Test connection with 10 second timeout
                let timeout_duration = std::time::Duration::from_secs(10);

                let result = tokio::time::timeout(
                    timeout_duration,
                    test_database_connection(config, password),
                )
                .await;

                match result {
                    Ok(Ok(success)) => {
                        if success {
                            Ok(())
                        } else {
                            Err("Connection failed: Unable to connect to database".to_string())
                        }
                    }
                    Ok(Err(e)) => Err(format!("Connection error: {}", e)),
                    Err(_) => {
                        Err("Connection timeout: Failed to connect within 10 seconds".to_string())
                    }
                }
            },
            Message::ConnectionTestResult,
        )
    }

    fn view(&self) -> Element<'_, Message> {
        // Main layout: menu bar, content area, status bar (vertical stack)
        let layout = column![
            self.menu_bar(),
            self.content_area(),
            self.render_status_bar(),
        ]
        .spacing(0);

        container(layout)
            .width(Fill)
            .height(Fill)
            .style(move |_theme| container::Style {
                background: Some(self.theme.background.into()),
                ..Default::default()
            })
            .into()
    }

    fn menu_bar(&self) -> Element<'_, Message> {
        self.menu_bar.view(
            self.open_menu,
            &self.view_registry,
            Message::MenuToggle,
            Message::MenuAction,
            Message::CloseMenu,
        )
    }

    fn content_area(&self) -> Element<'_, Message> {
        let mut content_row = row![].spacing(0);

        // Conditionally show left panel
        if self.show_left_panel {
            content_row = content_row.push(self.left_panel());
        }

        content_row = content_row.push(self.main_panel());

        container(content_row).width(Fill).height(Fill).into()
    }

    fn left_panel(&self) -> Element<'_, Message> {
        self.left_panel.view(
            self.panel_width,
            self.active_component,
            &self.view_registry,
            Message::LeftPanelTabClicked,
            Message::ResizeStart,
        )
    }

    fn main_panel(&self) -> Element<'_, Message> {
        if self.showing_settings_editor {
            self.settings_editor.view(
                &self.settings_editor_data,
                &self.settings_validation_error,
                &self.settings_success_message,
                Message::SettingsEditor,
            )
        } else if self.showing_adapter_selector {
            self.adapter_selector
                .view(&self.selected_adapter, Message::AdapterSelector)
        } else if self.showing_connection_form {
            // Show the appropriate adapter-specific form
            match self.selected_adapter {
                Some(DatabaseType::Postgres) => self.postgres_form.view(
                    &self.postgres_form_data,
                    self.postgres_testing,
                    &self.postgres_test_result,
                    Message::PostgresForm,
                ),
                Some(DatabaseType::MySQL) => self.mysql_form.view(
                    &self.mysql_form_data,
                    self.mysql_testing,
                    &self.mysql_test_result,
                    Message::MySQLForm,
                ),
                Some(DatabaseType::SQLite) => self.sqlite_form.view(
                    &self.sqlite_form_data,
                    self.sqlite_testing,
                    &self.sqlite_test_result,
                    Message::SQLiteForm,
                ),
                Some(DatabaseType::MongoDB) => self.mongodb_form.view(
                    &self.mongodb_form_data,
                    self.mongodb_testing,
                    &self.mongodb_test_result,
                    Message::MongoDBForm,
                ),
                Some(DatabaseType::SQLServer) => self.sqlserver_form.view(
                    &self.sqlserver_form_data,
                    self.sqlserver_testing,
                    &self.sqlserver_test_result,
                    Message::SQLServerForm,
                ),
                Some(DatabaseType::Oracle) => self.oracle_form.view(
                    &self.oracle_form_data,
                    self.oracle_testing,
                    &self.oracle_test_result,
                    Message::OracleForm,
                ),
                Some(DatabaseType::DuckDB) => unreachable!("DuckDB not used in rusty-app"),
                None => {
                    // Fallback to old connection form if adapter not selected
                    self.connection_form
                        .view(&self.connection_form_data, Message::ConnectionForm)
                }
            }
        } else {
            self.main_panel.view(
                Message::NewMainTab,
                Message::MainTabClicked,
                Message::MainTabClosed,
                Message::QueryEditor,
                &self.query_results,
                &self.query_errors,
            )
        }
    }

    fn render_status_bar(&self) -> Element<'_, Message> {
        self.status_bar.view(&self.connection_status)
    }

    /// Execute query for the given tab
    fn execute_query(&mut self, tab_id: TabId) -> Task<Message> {
        // Check if already executing
        if self.executing_query {
            warn!("Query execution already in progress");
            return Task::none();
        }

        // Get query text from the editor
        let query_text = match self.main_panel.get_query_text(tab_id) {
            Some(text) => {
                if text.trim().is_empty() {
                    self.query_errors
                        .insert(tab_id, Some("Query is empty".to_string()));
                    return Task::none();
                }
                text
            }
            None => {
                warn!("Tab {} not found or has no query editor", tab_id);
                self.query_errors
                    .insert(tab_id, Some("Query editor not found".to_string()));
                return Task::none();
            }
        };

        // Check if there's an active connection
        let connection_id = match self.connection_manager.selected_id() {
            Some(id) => id.to_string(),
            None => {
                // No active connection
                self.query_errors.insert(tab_id, Some("No active database connection selected. Please connect to a database first.".to_string()));
                return Task::none();
            }
        };

        // Set executing flag
        self.executing_query = true;
        info!(
            "Executing query for tab {} on connection {}",
            tab_id, connection_id
        );

        // Execute query with timeout
        // NOTE: This is a placeholder - actual implementation would need to:
        // 1. Get the connection from connection_manager
        // 2. Call execute_query on the adapter
        // 3. Handle the result
        // For now, we'll return a sample result to test the UI
        Task::perform(
            async move {
                use std::time::Duration;

                // Simulate query execution with timeout
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Return a sample result for testing
                Ok(QueryResult {
                    columns: vec!["id".to_string(), "name".to_string(), "email".to_string()],
                    rows: vec![
                        vec![
                            arni::QueryValue::Int(1),
                            arni::QueryValue::Text("Alice".to_string()),
                            arni::QueryValue::Text("alice@example.com".to_string()),
                        ],
                        vec![
                            arni::QueryValue::Int(2),
                            arni::QueryValue::Text("Bob".to_string()),
                            arni::QueryValue::Text("bob@example.com".to_string()),
                        ],
                    ],
                    rows_affected: Some(2),
                })
            },
            move |result| Message::QueryExecutionResult(tab_id, result),
        )
    }
}
