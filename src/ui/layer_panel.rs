use gtk4::prelude::*;
use gtk4::{Box, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Switch};
use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::layer::{Layer, LayerManager};

type LayerCallback = std::boxed::Box<dyn Fn(usize)>;
type VisibilityCallback = std::boxed::Box<dyn Fn(usize, bool)>;

pub struct LayerPanel {
    container: ScrolledWindow,
    list_box: ListBox,
    layer_manager: RefCell<Option<Rc<LayerManager>>>,
    on_layer_selected: Rc<RefCell<Option<LayerCallback>>>,
    on_visibility_changed: Rc<RefCell<Option<VisibilityCallback>>>,
}

impl LayerPanel {
    pub fn new() -> Self {
        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        
        let container = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&list_box)
            .width_request(200)
            .build();
        
        let this = Self {
            container,
            list_box,
            layer_manager: RefCell::new(None),
            on_layer_selected: Rc::new(RefCell::new(None)),
            on_visibility_changed: Rc::new(RefCell::new(None)),
        };
        
        this.setup_signals();
        this
    }
    
    fn setup_signals(&self) {
        let on_selected = self.on_layer_selected.clone();
        
        self.list_box.connect_selected_rows_changed(move |list_box| {
            if let Some(row) = list_box.selected_row() {
                // Get layer id from row
                if let Some(widget) = row.child() {
                    if let Ok(label) = widget.downcast::<Label>() {
                        let text = label.label();
                        // Parse id from text (format: "id: Name")
                        if let Some(id_str) = text.split(':').next() {
                            if let Ok(id) = id_str.parse::<usize>() {
                                if let Some(ref callback) = *on_selected.borrow() {
                                    callback(id);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    
    pub fn set_layer_manager(&self, manager: Rc<LayerManager>) {
        *self.layer_manager.borrow_mut() = Some(manager.clone());
        self.refresh_layers();
    }
    
    pub fn refresh_layers(&self) {
        // Clear existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        // Add layer rows
        if let Some(ref manager) = *self.layer_manager.borrow() {
            let layers = manager.get_all_layers();
            for layer in layers {
                let row = self.create_layer_row(&layer);
                self.list_box.append(&row);
            }
        }
    }
    
    fn create_layer_row(&self, layer: &Rc<Layer>) -> ListBoxRow {
        let hbox = Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);
        
        // Visibility switch
        let visibility_switch = Switch::new();
        visibility_switch.set_active(layer.is_visible());
        visibility_switch.set_valign(gtk4::Align::Center);
        
        let layer_id = layer.id;
        let on_visibility = self.on_visibility_changed.clone();
        let manager_ref = self.layer_manager.clone();
        
        visibility_switch.connect_active_notify(move |switch| {
            let is_visible = switch.is_active();
            if let Some(ref manager) = *manager_ref.borrow() {
                if let Some(layer) = manager.get_layer(layer_id) {
                    layer.set_visible(is_visible);
                }
            }
            if let Some(ref callback) = *on_visibility.borrow() {
                callback(layer_id, is_visible);
            }
        });
        
        // Layer name label
        let label = Label::new(Some(&format!("{}: {}", layer.id, layer.name)));
        label.set_halign(gtk4::Align::Start);
        label.set_hexpand(true);
        
        hbox.append(&visibility_switch);
        hbox.append(&label);
        
        let row = ListBoxRow::new();
        row.set_child(Some(&hbox));
        row
    }
    
    pub fn on_layer_selected<F: Fn(usize) + 'static>(&self, callback: F) {
        *self.on_layer_selected.borrow_mut() = Some(std::boxed::Box::new(callback));
    }
    
    pub fn on_visibility_changed<F: Fn(usize, bool) + 'static>(&self, callback: F) {
        *self.on_visibility_changed.borrow_mut() = Some(std::boxed::Box::new(callback));
    }
    
    pub fn widget(&self) -> &ScrolledWindow {
        &self.container
    }
    
    pub fn add_layer_controls(&self, parent_box: &Box) {
        // Add layer action buttons
        let controls_box = Box::new(Orientation::Horizontal, 4);
        controls_box.set_halign(gtk4::Align::Center);
        controls_box.set_margin_top(8);
        controls_box.set_margin_bottom(8);
        
        let add_btn = Button::with_label("+");
        add_btn.set_tooltip_text(Some("Add new layer"));
        
        let delete_btn = Button::with_label("-");
        delete_btn.set_tooltip_text(Some("Delete selected layer"));
        
        let dup_btn = Button::with_label("Dup");
        dup_btn.set_tooltip_text(Some("Duplicate selected layer"));
        
        let manager_ref = self.layer_manager.clone();
        let panel_ref = self.list_box.downgrade();
        
        add_btn.connect_clicked(move |_btn| {
            if let Some(ref manager) = *manager_ref.borrow() {
                manager.add_layer("New Layer");
                // Refresh would be called via signal
            }
        });
        
        let manager_ref2 = self.layer_manager.clone();
        delete_btn.connect_clicked(move |_btn| {
            if let Some(ref manager) = *manager_ref2.borrow() {
                if let Some(active) = manager.get_active_layer() {
                    manager.delete_layer(active.id);
                }
            }
        });
        
        let manager_ref3 = self.layer_manager.clone();
        dup_btn.connect_clicked(move |_btn| {
            if let Some(ref manager) = *manager_ref3.borrow() {
                if let Some(active) = manager.get_active_layer() {
                    manager.duplicate_layer(active.id);
                }
            }
        });
        
        controls_box.append(&add_btn);
        controls_box.append(&delete_btn);
        controls_box.append(&dup_btn);
        
        parent_box.append(&controls_box);
    }
}

impl Default for LayerPanel {
    fn default() -> Self {
        Self::new()
    }
}
