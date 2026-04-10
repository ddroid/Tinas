use gtk4::prelude::*;
use gtk4::{Box, Button, Dialog, Label, ListBox, ListBoxRow, Orientation, Picture, ScrolledWindow};
use std::rc::Rc;
use std::cell::RefCell;

use crate::temp_db::{ImageVersion, TempDb};

type RestoreCallback = std::boxed::Box<dyn Fn(&[u8]) + 'static>;

pub struct VersionHistoryDialog {
    dialog: Dialog,
    resource_id: u16,
    pak_file: String,
    temp_db: Rc<RefCell<TempDb>>,
    on_restore: Rc<RefCell<Option<RestoreCallback>>>,
}

impl VersionHistoryDialog {
    pub fn new(
        parent: &impl IsA<gtk4::Window>,
        resource_id: u16,
        pak_file: &str,
        temp_db: Rc<RefCell<TempDb>>,
    ) -> Self {
        let dialog = Dialog::new();
        dialog.set_title(Some(&format!("Version History - Resource {}", resource_id)));
        dialog.set_default_size(500, 400);
        dialog.set_modal(true);
        dialog.set_transient_for(Some(parent));
        
        let content_area = dialog.content_area();
        content_area.set_margin_top(12);
        content_area.set_margin_bottom(12);
        content_area.set_margin_start(12);
        content_area.set_margin_end(12);
        
        // Title
        let title = Label::new(Some("Version History"));
        title.add_css_class("title-2");
        content_area.append(&title);
        
        // Subtitle
        let subtitle = Label::new(Some(&format!("Resource {} from {}", resource_id, pak_file)));
        subtitle.add_css_class("dim-label");
        content_area.append(&subtitle);
        
        // Scrollable list
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .margin_top(12)
            .build();
        
        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::None);
        scrolled.set_child(Some(&list_box));
        content_area.append(&scrolled);
        
        // Close button - must be added before moving dialog
        dialog.add_button("Close", gtk4::ResponseType::Close);
        dialog.connect_response(|dlg, response| {
            if response == gtk4::ResponseType::Close {
                dlg.close();
            }
        });
        
        // Load versions
        let this = Self {
            dialog,
            resource_id,
            pak_file: pak_file.to_string(),
            temp_db: temp_db.clone(),
            on_restore: Rc::new(RefCell::new(None)),
        };
        
        this.populate_versions(&list_box);
        
        this
    }
    
    fn populate_versions(&self, list_box: &ListBox) {
        let db = self.temp_db.borrow();
        
        if let Some(versions) = db.get_versions(&self.pak_file, self.resource_id) {
            if versions.is_empty() {
                let label = Label::new(Some("No version history available.\nMake some edits to start tracking versions."));
                label.add_css_class("dim-label");
                label.set_wrap(true);
                label.set_justify(gtk4::Justification::Center);
                list_box.append(&label);
                return;
            }
            
            // Show versions in reverse order (newest first)
            for (i, version) in versions.iter().enumerate().rev() {
                let row = self.create_version_row(version, i == versions.len() - 1);
                list_box.append(&row);
            }
        } else {
            let label = Label::new(Some("No version history available."));
            label.add_css_class("dim-label");
            list_box.append(&label);
        }
    }
    
    fn create_version_row(&self, version: &ImageVersion, is_current: bool) -> ListBoxRow {
        let hbox = Box::new(Orientation::Horizontal, 12);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);
        
        // Info box
        let info_box = Box::new(Orientation::Vertical, 4);
        info_box.set_hexpand(true);
        
        // Timestamp and status
        let time_str = chrono::DateTime::from_timestamp(version.timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());
        
        let status = if is_current {
            " (Current)"
        } else if version.is_saved {
            " (Saved)"
        } else {
            " (Unsaved)"
        };
        
        let time_label = Label::new(Some(&format!("{}{}", time_str, status)));
        if is_current {
            time_label.add_css_class("heading");
        }
        info_box.append(&time_label);
        
        // Action description
        let action_label = Label::new(Some(&version.action_description));
        action_label.add_css_class("dim-label");
        action_label.set_halign(gtk4::Align::Start);
        info_box.append(&action_label);
        
        hbox.append(&info_box);
        
        // Preview thumbnail if possible
        // Try to load the image data as a Pixbuf
        let bytes = glib::Bytes::from(&version.image_data);
        let stream = gtk4::gio::MemoryInputStream::from_bytes(&bytes);
        if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_stream(&stream, gtk4::gio::Cancellable::NONE) {
            // Convert Pixbuf to Texture (which implements Paintable)
            let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
            let picture = Picture::for_paintable(&texture);
            picture.set_size_request(64, 64);
            picture.set_can_shrink(false);
            hbox.append(&picture);
        }
        
        // Restore button (if not current)
        if !is_current {
            let restore_btn = Button::with_label("Restore");
            restore_btn.add_css_class("suggested-action");
            
            let temp_db = self.temp_db.clone();
            let pak_file = self.pak_file.clone();
            let resource_id = self.resource_id;
            let version_id = version.id.clone();
            let on_restore = self.on_restore.clone();
            
            restore_btn.connect_clicked(move |_| {
                if let Some(data) = temp_db.borrow().get_version(&pak_file, resource_id, &version_id) {
                    if let Some(ref callback) = *on_restore.borrow() {
                        callback(&data.image_data);
                    }
                }
            });
            
            hbox.append(&restore_btn);
        }
        
        let row = ListBoxRow::new();
        row.set_child(Some(&hbox));
        row
    }
    
    pub fn set_on_restore<F>(&self, callback: F) 
    where
        F: Fn(&[u8]) + 'static,
    {
        let cb: RestoreCallback = std::boxed::Box::new(callback);
        *self.on_restore.borrow_mut() = Some(cb);
    }
    
    pub fn show(&self) {
        self.dialog.show();
    }
}
