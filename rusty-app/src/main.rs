use iced::widget::{column, container, row};
use iced::{Element, Fill, Subscription, Task, Theme};
use iced::event::Event;
use iced::mouse;
use rusty_app::adapter_selector::{AdapterSelector, AdapterSelectorMessage};
use rusty_app::components::{ComponentAction, ComponentId, PropertiesComponent, ServerListComponent, TableListComponent};
use rusty_app::connection_form::{ConnectionForm, ConnectionFormData, ConnectionFormMessage};
use rusty_app::container::ContainerManager;
use rusty_app::container::converter::sync_connections_with_containers;
use rusty_app::left_panel::{self, LeftPanel};
use rusty_app::main_panel::{MainPanel, TabId};
use rusty_app::menu_bar::{MenuBar, MenuAction, MenuItem};
use rusty_app::mysql_connection_form::{MySQLConnectionForm, MySQLConnectionFormData, MySQLConnectionFormMessage};
use rusty_app::postgres_connection_form::{PostgresConnectionForm, PostgresConnectionFormData, PostgresConnectionFormMessage};
use rusty_app::settings::{logging::build_logging_config, SettingsManager};
use rusty_app::settings_editor::{SettingsEditor, SettingsEditorData, SettingsEditorMessage};
use rusty_app::sqlite_connection_form::{SQLiteConnectionForm, SQLiteConnectionFormData, SQLiteConnectionFormMessage};
use rusty_app::status_bar::{ConnectionStatus, StatusBar};
use rusty_app::theme::ThemeColors;
use rusty_app::views::{RegionId, ViewRegistry};
use rusty_data::adapter::{ConnectionConfig, DatabaseType};
use rusty_data::config::ConfigManager;
use std::collections::HashMap;
use tracing::{info, warn};

pub fn main() -> iced::Result {
    // Load settings and configure logging
    let settings_manager = SettingsManager::new("~/.rusty-app")
        .expect("Failed to initialize settings manager");

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
    config_manager: ConfigManager,
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
}

impl Default for DatabaseIDE {
    fn default() -> Self {
        // Initialize SettingsManager
        let mut settings_manager = SettingsManager::new("~/.rusty-app")
            .expect("Failed to initialize settings manager");

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

        // Initialize ConfigManager (kept for validation methods)
        let config_manager = ConfigManager::new("~/.rusty-app")
            .expect("Failed to initialize config manager");

        // Migrate connections from connections.toml to settings.toml if needed
        let connections_file = config_manager.connections_file();
        if connections_file.exists() && settings_manager.settings().connections.is_empty() {
            info!("Migrating connections from connections.toml to settings.toml");
            match config_manager.load_connections() {
                Ok(old_connections) => {
                    if !old_connections.is_empty() {
                        // Save to settings.toml
                        settings_manager.settings_mut().connections = old_connections.clone();
                        if let Err(e) = settings_manager.save() {
                            warn!("Failed to save migrated connections: {}", e);
                        } else {
                            info!(count = old_connections.len(), "Migrated connections to settings.toml");
                            // Delete old connections.toml after successful migration
                            if let Err(e) = std::fs::remove_file(&connections_file) {
                                warn!("Failed to delete old connections.toml: {}", e);
                            } else {
                                info!("Removed old connections.toml after migration");
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load connections for migration: {}", e);
                }
            }
        }

        // Initialize ContainerManager
        let container_manager = ContainerManager::new("rusty-data");

        // Load saved connections from settings.toml
        let mut saved_connections = settings_manager.settings().connections.clone();

        info!(count = saved_connections.len(), "Loaded saved connections from settings");

        // Sync connections with running containers
        if container_manager.is_podman_available() {
            match container_manager.list_containers() {
                Ok(containers) => {
                    let running_containers: Vec<_> = containers
                        .iter()
                        .filter(|c| c.is_running())
                        .cloned()
                        .collect();

                    info!(
                        count = running_containers.len(),
                        "Found running containers"
                    );

                    // Sync connections with container state
                    saved_connections = sync_connections_with_containers(
                        &running_containers,
                        &saved_connections,
                    );

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
            config_manager,
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
        }
    }
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
    NewConnection,
    ConnectionForm(ConnectionFormMessage),
    ConnectionTestResult(Result<(), String>),
    ComponentAction(ComponentAction),
    ShowSettings,
    SettingsEditor(SettingsEditorMessage),
}

impl From<ComponentAction> for Message {
    fn from(action: ComponentAction) -> Self {
        Message::ComponentAction(action)
    }
}

/// Test database connection with appropriate adapter
async fn test_database_connection(config: ConnectionConfig, password: Option<String>) -> rusty_data::Result<bool> {
    use rusty_data::adapter::DatabaseAdapter;
    use rusty_data::adapters;

    let password_ref = password.as_deref();

    match config.db_type {
        DatabaseType::Postgres => {
            let adapter = adapters::postgres::PostgresAdapter::new();
            adapter.test_connection(&config, password_ref).await
        }
        DatabaseType::MySQL => {
            let adapter = adapters::mysql::MySqlAdapter::new();
            adapter.test_connection(&config, password_ref).await
        }
        DatabaseType::SQLite => {
            let adapter = adapters::sqlite::SqliteAdapter::new();
            adapter.test_connection(&config, password_ref).await
        }
        DatabaseType::MongoDB | DatabaseType::SQLServer | DatabaseType::Oracle => {
            Err(rusty_data::error::DataError::Config(
                format!("Database type {:?} is not yet supported", config.db_type)
            ))
        }
    }
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
                                self.settings_editor_data = SettingsEditorData::from_manager(&self.settings_manager);
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
                self.main_panel
                    .add_tab(format!("Query {}", tab_count));
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
            Message::NewConnection => {
                self.showing_connection_form = true;
                self.connection_form_data = ConnectionFormData::new();
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
                        self.settings_manager.settings_mut().connections = self.saved_connections.clone();
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
                        self.connection_form_data = self.connection_form_data.clone().with_db_type(db_type);
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
                self.settings_editor_data = SettingsEditorData::from_manager(&self.settings_manager);
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
                        self.settings_editor_data.logging.console_filter = if value.is_empty() {
                            None
                        } else {
                            Some(value)
                        };
                        Task::none()
                    }
                    SettingsEditorMessage::FileFilterChanged(value) => {
                        self.settings_editor_data.logging.file_filter = if value.is_empty() {
                            None
                        } else {
                            Some(value)
                        };
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
                        self.settings_editor_data.logging.console_enabled = !self.settings_editor_data.logging.console_enabled;
                        Task::none()
                    }
                    SettingsEditorMessage::FileFormatChanged(format) => {
                        self.settings_editor_data.logging.file_format = format;
                        Task::none()
                    }
                    SettingsEditorMessage::FileEnabledToggled => {
                        self.settings_editor_data.logging.file_enabled = !self.settings_editor_data.logging.file_enabled;
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
                        self.settings_editor_data.ui_preferences.show_left_panel = !self.settings_editor_data.ui_preferences.show_left_panel;
                        Task::none()
                    }
                    SettingsEditorMessage::Save => {
                        // Update settings_manager with edited values
                        self.settings_manager.settings_mut().logging = self.settings_editor_data.logging.clone();
                        self.settings_manager.settings_mut().ui_preferences = self.settings_editor_data.ui_preferences.clone();

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
                        if let Ok(logging_config) = build_logging_config(&self.settings_manager.settings().logging) {
                            if let Err(e) = logging_config.apply() {
                                self.settings_validation_error = Some(format!("Failed to reload logging: {}", e));
                                self.settings_success_message = None;
                                return Task::none();
                            }
                        } else {
                            self.settings_validation_error = Some("Invalid logging configuration".to_string());
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
                        self.settings_success_message = Some("Settings saved successfully".to_string());
                        self.settings_validation_error = None;
                        self.showing_settings_editor = false;

                        Task::none()
                    }
                    SettingsEditorMessage::Reload => {
                        // Reload settings from disk
                        if let Err(e) = self.settings_manager.reload() {
                            self.settings_validation_error = Some(format!("Failed to reload: {}", e));
                            self.settings_success_message = None;
                            return Task::none();
                        }

                        // Reset editor data from reloaded settings
                        self.settings_editor_data = SettingsEditorData::from_manager(&self.settings_manager);

                        // Clear errors and show success
                        self.settings_validation_error = None;
                        self.settings_success_message = Some("Settings reloaded from disk".to_string());

                        // Keep settings editor open (don't close)
                        Task::none()
                    }
                }
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
        use rusty_data::adapter::DatabaseType;

        // Generate a unique ID from the connection name
        let id = form_data.name.to_lowercase().replace(' ', "-");

        // For SQLite, database field holds the file path
        let (host, port, database) = match form_data.db_type {
            DatabaseType::SQLite => (None, None, form_data.file_path.clone()),
            _ => {
                let port = form_data.port.parse::<u16>().ok();
                (Some(form_data.host.clone()), port, form_data.database.clone())
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
                    test_database_connection(config, password)
                ).await;

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
            },
            Message::ConnectionTestResult
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

        container(content_row)
            .width(Fill)
            .height(Fill)
            .into()
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
        } else if self.showing_connection_form {
            self.connection_form.view(
                &self.connection_form_data,
                Message::ConnectionForm,
            )
        } else {
            self.main_panel.view(
                Message::NewMainTab,
                Message::MainTabClicked,
                Message::MainTabClosed,
            )
        }
    }

    fn render_status_bar(&self) -> Element<'_, Message> {
        self.status_bar.view(self.connection_status)
    }
}
