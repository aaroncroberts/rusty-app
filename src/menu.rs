// src/menu.rs

use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CssProvider, Image, Label, Orientation,
    STYLE_PROVIDER_PRIORITY_APPLICATION, Widget,
};

pub struct MenuBar {
    container: GtkBox,
}

impl MenuBar {
    pub fn new() -> Self {
        // --- CSS for dark menu styling ---
        let css = "
            .menu-bar {
                background-color: #23272e;
            }
            .menu-btn {
                background: none;
                border: none;
                color: #f5f6fa;
                padding: 8px 18px;
                border-radius: 8px;
                font-size: 1rem;
                font-weight: 500;
                transition: background 0.2s;
            }
            .menu-btn:hover {
                background: #353b45;
                color: #ffffff;
            }
            .menu-btn image {
                color: #f5f6fa;
            }
        ";
        let provider = CssProvider::new();
        provider.load_from_data(css);
        let display = Display::default().expect("Failed to get default display");
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let container = GtkBox::new(Orientation::Horizontal, 0);
        container.set_hexpand(true);
        container.set_vexpand(false);
        container.add_css_class("menu-bar");

        // Menu item labels and icons
        let items = [
            ("File", "document-open-symbolic"),
            ("Edit", "edit-symbolic"),
            ("View", "view-list-symbolic"),
            ("Help", "help-browser-symbolic"),
        ];

        for (label, icon_name) in items {
            let btn = Button::new();
            btn.set_hexpand(false);
            btn.set_halign(Align::Start);
            btn.set_margin_top(6);
            btn.set_margin_bottom(6);
            btn.set_margin_start(6);
            btn.set_margin_end(6);
            btn.add_css_class("menu-btn");

            // Create a horizontal box for icon + label
            let btn_box = GtkBox::new(Orientation::Horizontal, 6);

            let icon = Image::from_icon_name(icon_name);
            icon.set_pixel_size(18);

            let lbl = Label::new(Some(label));
            lbl.set_halign(Align::Center);

            btn_box.append(&icon);
            btn_box.append(&lbl);

            btn.set_child(Some(&btn_box));

            container.append(&btn);
        }

        // Add a spacer to push items to the left
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        container.append(&spacer);

        Self { container }
    }

    pub fn widget(&self) -> &impl IsA<Widget> {
        &self.container
    }
}
