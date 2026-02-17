use iced::widget::{column, container, row, text};
use iced::{Center, Element, Fill, Task, Theme};
use rusty_app::left_panel::{self, LeftPanel, PanelTab};
use rusty_app::menu_bar::{MenuBar, MenuItem};
use rusty_app::theme::ThemeColors;

pub fn main() -> iced::Result {
    iced::application("Database IDE v0.0.1", DatabaseIDE::update, DatabaseIDE::view)
        .theme(|_| Theme::TokyoNightStorm)
        .window_size((1280.0, 800.0))
        .run()
}

struct DatabaseIDE {
    theme: ThemeColors,
    menu_bar: MenuBar,
    left_panel: LeftPanel,
    panel_width: f32,
    active_tab: PanelTab,
    is_resizing: bool,
}

impl Default for DatabaseIDE {
    fn default() -> Self {
        let theme = ThemeColors::dark();
        Self {
            theme,
            menu_bar: MenuBar::new(theme),
            left_panel: LeftPanel::new(theme),
            panel_width: left_panel::DEFAULT_WIDTH,
            active_tab: PanelTab::default(),
            is_resizing: false,
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
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Main layout with themed background
        let layout = column![
            self.menu_bar(),
            self.content_area(),
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
        )
    }

    fn main_panel(&self) -> Element<'_, Message> {
        let panel = container(
            column![
                text("Query Editor").size(16).color(self.theme.text),
                text("Ready to execute queries...").size(12).color(self.theme.text_secondary),
            ]
            .spacing(20)
            .padding(20)
        )
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .align_x(Center)
        .align_y(Center)
        .style(move |_theme| container::Style {
            background: Some(self.theme.background.into()),
            ..Default::default()
        });

        panel.into()
    }
}
