use iced::widget::{column, container, row};
use iced::{Element, Fill, Task, Theme};
use rusty_app::left_panel::{self, LeftPanel, PanelTab};
use rusty_app::main_panel::{MainPanel, TabId};
use rusty_app::menu_bar::{MenuBar, MenuItem};
use rusty_app::status_bar::{ConnectionStatus, StatusBar};
use rusty_app::theme::ThemeColors;
use rusty_logging::LoggingConfig;
use tracing::info;

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
    panel_width: f32,
    active_tab: PanelTab,
    is_resizing: bool,
    connection_status: ConnectionStatus,
}

impl Default for DatabaseIDE {
    fn default() -> Self {
        let theme = ThemeColors::dark();
        let mut main_panel = MainPanel::new(theme);

        // Create an initial tab
        main_panel.add_tab("Query 1".to_string());

        Self {
            theme,
            menu_bar: MenuBar::new(theme),
            left_panel: LeftPanel::new(theme),
            main_panel,
            status_bar: StatusBar::new(theme),
            panel_width: left_panel::DEFAULT_WIDTH,
            active_tab: PanelTab::default(),
            is_resizing: false,
            connection_status: ConnectionStatus::default(),
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
                // TODO: Open connection editor in main panel
                println!("New connection button clicked");
                Task::none()
            }
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
            Message::TabClicked,
            Message::ResizeStart,
            Message::NewConnection,
        )
    }

    fn main_panel(&self) -> Element<'_, Message> {
        self.main_panel.view(
            Message::NewMainTab,
            Message::MainTabClicked,
            Message::MainTabClosed,
        )
    }

    fn render_status_bar(&self) -> Element<'_, Message> {
        self.status_bar.view(self.connection_status)
    }
}
