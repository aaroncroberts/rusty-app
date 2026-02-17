use iced::widget::{column, container, row, text};
use iced::{Center, Element, Fill, Task, Theme};
use rusty_app::theme::ThemeColors;

pub fn main() -> iced::Result {
    iced::application("Database IDE v0.1.0", DatabaseIDE::update, DatabaseIDE::view)
        .theme(|_| Theme::TokyoNightStorm) // Dark theme
        .window_size((1280.0, 800.0))
        .run()
}

#[derive(Default)]
struct DatabaseIDE {
    // Application state will go here
}

#[derive(Debug, Clone)]
enum Message {
    // Messages will go here
}

impl DatabaseIDE {
    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // Main layout: vertical stack with menu bar + content area
        let layout = column![
            // Menu Bar (top)
            self.menu_bar(),
            // Content Area (left panel + main panel)
            self.content_area(),
        ]
        .spacing(0);

        container(layout)
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn menu_bar(&self) -> Element<'_, Message> {
        // Placeholder for menu bar
        let menu = container(
            row![
                text("File").size(14),
                text("Edit").size(14),
                text("View").size(14),
                text("Tools").size(14),
            ]
            .spacing(20)
            .padding(10)
        )
        .width(Fill);

        menu.into()
    }

    fn content_area(&self) -> Element<'_, Message> {
        // Horizontal layout: left panel + main panel
        let content = row![
            // Left Panel (object viewer)
            self.left_panel(),
            // Main Panel (query editor / results)
            self.main_panel(),
        ]
        .spacing(0);

        container(content)
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn left_panel(&self) -> Element<'_, Message> {
        // Placeholder for left panel with tabs
        let panel = container(
            column![
                text("Servers").size(12),
                text("─────────").size(10),
                text("(No connections)").size(11),
            ]
            .spacing(10)
            .padding(15)
        )
        .width(200); // Fixed width for left panel

        panel.into()
    }

    fn main_panel(&self) -> Element<'_, Message> {
        // Placeholder for main panel
        let panel = container(
            column![
                text("Query Editor").size(16),
                text("Ready to execute queries...").size(12),
            ]
            .spacing(20)
            .padding(20)
        )
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .align_x(Center)
        .align_y(Center);

        panel.into()
    }
}
