// src/main.rs

mod mailbox;
mod menu;

use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, CssProvider, Label, Orientation,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use mailbox::{Folder, Mailbox};
use menu::MenuBar;

fn main() {
    let app = Application::builder()
        .application_id("com.example.rusty-app")
        .build();

    app.connect_activate(build_ui);

    app.run();
}

fn build_ui(app: &Application) {
    // --- CSS for dark theme ---
    let css = "
        window, .main-bg {
            background-color: #23272e;
            color: #f5f6fa;
        }
        .status-bar {
            background-color: #2d323b;
            color: #f5f6fa;
        }
        .menu-bar {
            background-color: #23272e;
            color: #f5f6fa;
        }
        .mailbox {
            background-color: #23272e;
        }
        .content-area {
            background-color: #23272e;
        }
        button.flat {
            background: none;
            border: none;
            color: #f5f6fa;
        }
        button.flat:hover {
            background: #353b45;
        }
        list {
            background-color: #23272e;
        }
        list row:selected {
            background-color: #353b45;
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

    let window = ApplicationWindow::builder()
        .application(app)
        .title("rusty-app: v0.0.1")
        .default_width(900)
        .default_height(600)
        .build();

    // Main vertical box (3 rows)
    let vbox = GtkBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("main-bg");

    // --- Row 1: Menu Bar ---
    let menu_bar = MenuBar::new();
    let menu_widget = menu_bar.widget();
    menu_widget.add_css_class("menu-bar");
    vbox.append(menu_widget);

    // --- Row 2: Main Layout Area (3 columns) ---
    let main_area = GtkBox::new(Orientation::Horizontal, 0);
    main_area.set_hexpand(true);
    main_area.set_vexpand(true);
    main_area.add_css_class("content-area");

    // Mailbox folders
    let folders = vec![
        Folder {
            name: "Inbox".to_string(),
        },
        Folder {
            name: "Sent".to_string(),
        },
        Folder {
            name: "Drafts".to_string(),
        },
        Folder {
            name: "Spam".to_string(),
        },
        Folder {
            name: "Trash".to_string(),
        },
    ];
    let mailbox = Mailbox::new(&folders);
    let mailbox_widget = mailbox.widget();
    mailbox_widget.add_css_class("mailbox");

    // Placeholder for Message List (center column)
    let message_list_area = GtkBox::new(Orientation::Vertical, 10);
    message_list_area.set_hexpand(true);
    message_list_area.set_vexpand(true);
    let message_list_label = Label::new(Some("Message List"));
    message_list_label.set_halign(Align::Center);
    message_list_label.set_valign(Align::Start);
    message_list_area.append(&message_list_label);

    // Placeholder for Content Display (right column)
    let content_display_area = GtkBox::new(Orientation::Vertical, 10);
    content_display_area.set_hexpand(false);
    content_display_area.set_vexpand(true);
    content_display_area.set_width_request(90);
    let content_display_label = Label::new(Some("Content Display"));
    content_display_label.set_halign(Align::Center);
    content_display_label.set_valign(Align::Start);
    content_display_area.append(&content_display_label);

    main_area.append(mailbox_widget);
    main_area.append(&message_list_area);
    main_area.append(&content_display_area);

    vbox.append(&main_area);

    // --- Row 3: Status Bar ---
    let status_bar = GtkBox::new(Orientation::Horizontal, 5);
    status_bar.set_hexpand(true);
    status_bar.add_css_class("status-bar");

    let status_label = Label::new(Some("Status: Ready"));
    status_label.set_halign(Align::Start);
    status_bar.append(&status_label);

    vbox.append(&status_bar);

    window.set_child(Some(&vbox));
    window.present();
}
