// src/mailbox.rs

use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CssProvider, Image, Label, ListBox, ListBoxRow, Orientation,
    STYLE_PROVIDER_PRIORITY_APPLICATION, Widget,
};

#[derive(Debug, Clone)]
pub struct Folder {
    pub name: String,
}

pub struct Mailbox {
    pub container: GtkBox,
}

impl Mailbox {
    pub fn new(folders: &[Folder]) -> Self {
        // --- CSS for dark mailbox styling ---
        let provider = CssProvider::new();
        provider.load_from_path("src/css/mailbox.css");

        let display = Display::default().expect("Failed to get default display");
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let container = GtkBox::new(Orientation::Vertical, 0);
        container.set_hexpand(false);
        container.set_vexpand(true);
        container.set_width_request(120);
        container.add_css_class("mailbox");

        let list = ListBox::new();
        list.set_vexpand(true);
        list.set_margin_top(8);
        list.set_margin_bottom(8);
        list.set_margin_start(4);
        list.set_margin_end(4);
        list.add_css_class("mailbox-list");

        for folder in folders {
            let row = ListBoxRow::new();
            row.add_css_class("mailbox-row");

            let row_box = GtkBox::new(Orientation::Horizontal, 6);
            row_box.set_margin_top(2);
            row_box.set_margin_bottom(2);
            row_box.set_margin_start(2);
            row_box.set_margin_end(2);

            // Smaller folder icon
            let icon = Image::from_icon_name("folder");
            icon.set_pixel_size(18);

            let label = Label::new(Some(&folder.name));
            label.set_xalign(0.0);
            label.set_margin_start(4);
            label.add_css_class("mailbox-label");

            row_box.append(&icon);
            row_box.append(&label);

            row.set_child(Some(&row_box));
            list.append(&row);
        }

        container.append(&list);

        Self { container }
    }

    pub fn widget(&self) -> &impl IsA<Widget> {
        &self.container
    }
}
