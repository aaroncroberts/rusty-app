use iced::widget::{column, container, row};
use iced::{Element, Fill, Subscription, Task, Theme};
use iced::event::Event;
use iced::mouse;
use rusty_app::components::ComponentAction;
use rusty_app::connection_form::{ConnectionForm, ConnectionFormData, ConnectionFormMessage};
use rusty_app::left_panel::{self, LeftPanel, PanelTab};
use rusty_app::main_panel::{MainPanel, TabId};
use rusty_app::menu_bar::{MenuBar, MenuAction, MenuItem};
use rusty_app::status_bar::{ConnectionStatus, StatusBar};
use rusty_app::theme::ThemeColors;
use rusty_app::views::ViewRegistry;
use rusty_data::adapter::{ConnectionConfig, DatabaseType};
use rusty_data::config::ConfigManager;
use rusty_logging::LoggingConfig;
use std::collections::HashMap;
use tracing::{info, warn};

pub fn main() -> iced::Result {
    // Initialize logging (fulfills rusty-data's logging needs via dependency inversion)
    LoggingConfig::builder()
        .with_console_compact()
        .with_console_filter("info")
        .with_file_text()
        .with_file_filter("debug")
        .with_file_directory("./logs")
        .with_file_prefix("rusty-app")
        .build()
        .expect("Invalid logging configuration")
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
    active_tab: PanelTab,
    is_resizing: bool,
    last_mouse_x: Option<f32>,
    connection_status: ConnectionStatus,
    config_manager: ConfigManager,
    saved_connections: Vec<ConnectionConfig>,
    showing_connection_form: bool,
    connection_form_data: ConnectionFormData,
    testing_connection: bool,
    test_result: Option<Result<(), String>>,
    open_menu: Option<MenuItem>,
    show_left_panel: bool,
    view_registry: ViewRegistry,
}

impl Default for DatabaseIDE {
    fn default() -> Self {
        let theme = ThemeColors::dark();
        let mut main_panel = MainPanel::new(theme);

        // Create an initial tab
        main_panel.add_tab("Query 1".to_string());

        // Initialize ConfigManager
        let config_manager = ConfigManager::new("~/.rusty-app")
            .expect("Failed to initialize config manager");

        // Load saved connections
        let saved_connections = config_manager
            .load_connections()
            .unwrap_or_else(|e| {
                warn!("Failed to load connections: {}", e);
                Vec::new()
            });

        info!(count = saved_connections.len(), "Loaded saved connections");

        Self {
            theme,
            menu_bar: MenuBar::new(theme),
            left_panel: LeftPanel::new(theme),
            main_panel,
            status_bar: StatusBar::new(theme),
            connection_form: ConnectionForm::new(theme),
            panel_width: left_panel::DEFAULT_WIDTH,
            active_tab: PanelTab::default(),
            is_resizing: false,
            last_mouse_x: None,
            connection_status: ConnectionStatus::default(),
            config_manager,
            saved_connections,
            showing_connection_form: false,
            connection_form_data: ConnectionFormData::new(),
            testing_connection: false,
            test_result: None,
            open_menu: None,
            show_left_panel: true,
            view_registry: ViewRegistry::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    MenuToggle(MenuItem),
    MenuAction(MenuAction),
    CloseMenu,
    TabClicked(PanelTab),
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
            Message::TabClicked(tab) => {
                self.active_tab = tab;
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

                        // Save to disk
                        if let Err(e) = self.config_manager.save_connections(&self.saved_connections) {
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
            self.active_tab,
            &self.saved_connections,
            Message::TabClicked,
            Message::ResizeStart,
            Message::NewConnection,
        )
    }

    fn main_panel(&self) -> Element<'_, Message> {
        if self.showing_connection_form {
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
