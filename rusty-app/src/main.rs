use iced::widget::{column, container, row};
use iced::{Element, Fill, Task, Theme};
use rusty_app::connection_form::{ConnectionForm, ConnectionFormData, ConnectionFormMessage};
use rusty_app::left_panel::{self, LeftPanel, PanelTab};
use rusty_app::main_panel::{MainPanel, TabId};
use rusty_app::menu_bar::{MenuBar, MenuItem};
use rusty_app::status_bar::{ConnectionStatus, StatusBar};
use rusty_app::theme::ThemeColors;
use rusty_data::adapter::ConnectionConfig;
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

    iced::application("Database IDE v0.0.1", DatabaseIDE::update, DatabaseIDE::view)
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
    connection_status: ConnectionStatus,
    config_manager: ConfigManager,
    saved_connections: Vec<ConnectionConfig>,
    showing_connection_form: bool,
    connection_form_data: ConnectionFormData,
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
            connection_status: ConnectionStatus::default(),
            config_manager,
            saved_connections,
            showing_connection_form: false,
            connection_form_data: ConnectionFormData::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    MenuItemClicked(MenuItem),
    TabClicked(PanelTab),
    ResizeStart,
    ResizeMove(f32),
    ResizeEnd,
    NewMainTab,
    MainTabClicked(TabId),
    MainTabClosed(TabId),
    NewConnection,
    ConnectionForm(ConnectionFormMessage),
    CancelConnectionForm,
}

impl DatabaseIDE {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MenuItemClicked(item) => {
                // Log menu item clicks (temporary implementation)
                println!("Menu item clicked: {:?}", item);
                Task::none()
            }
            Message::TabClicked(tab) => {
                self.active_tab = tab;
                Task::none()
            }
            Message::ResizeStart => {
                self.is_resizing = true;
                Task::none()
            }
            Message::ResizeMove(delta_x) => {
                if self.is_resizing {
                    self.panel_width = left_panel::constrain_width(self.panel_width + delta_x);
                }
                Task::none()
            }
            Message::ResizeEnd => {
                self.is_resizing = false;
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
                }
            }
            Message::CancelConnectionForm => {
                self.showing_connection_form = false;
                Task::none()
            }
        }
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
        self.menu_bar.view(Message::MenuItemClicked)
    }

    fn content_area(&self) -> Element<'_, Message> {
        let content = row![
            self.left_panel(),
            self.main_panel(),
        ]
        .spacing(0);

        container(content)
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
