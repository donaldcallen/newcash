// Copyright (C) 2018 Donald C. Allen
//
// This file is part of the Newcash Personal Finance Suite.
//
// Newcash is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// The Newcash Suite is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You have received a copy of the GNU General Public License
// along with the Newcash Suite.  It is also available at <http://www.gnu.org/licenses/>.

use calendar::display_calendar;
use constants::{Globals, RegisterCore, StockSplitsRegister, DATE_SIZE};
use gdk::enums::key;
use gdk::EventType::ButtonPress;
use gdk::{EventButton, EventKey, ModifierType};
use gtk::SelectionMode::Browse;
use gtk::TreeViewGridLines::Both;
use gtk::{
    CellRendererExt, CellRendererText, CellRendererTextExt, ContainerExt, GtkListStoreExt,
    GtkListStoreExtManual, GtkMenuExtManual, GtkMenuItemExt, GtkWindowExt, Inhibit, ListStore,
    Menu, MenuItem, MenuShellExt, ScrolledWindow, TreeModelExt, TreePath, TreeSelectionExt,
    TreeView, TreeViewColumn, TreeViewColumnExt, TreeViewExt, Type, WidgetExt, Window, WindowType,
    NONE_ADJUSTMENT,
};
use queries::{
    DELETE_STOCK_SPLIT_SQL, GET_SPLIT_FACTOR_SQL, NEW_STOCK_SPLIT_SQL, STOCK_SPLITS_REGISTER_SQL,
    STOCK_SPLIT_DATE_TODAY_SQL, STOCK_SPLIT_DATE_TO_END_OF_MONTH_SQL,
    STOCK_SPLIT_DATE_TO_FIRST_OF_MONTH_SQL, STOCK_SPLIT_DATE_TO_USER_ENTRY_SQL,
    STOCK_SPLIT_INCREMENT_DATE_SQL, UPDATE_SPLIT_FACTOR_SQL,
};
use std::rc::Rc;
use utilities::{
    column_index_to_column, create_tree_view_text_column, date_edited, display_message_dialog,
    get_selection_info, get_string_column_via_path, select_first_row, select_row,
    update_string_column_via_path,
};

// Columns in the stock splits store
const STORE_GUID: i32 = 0;
const STORE_DATE: i32 = STORE_GUID + 1;
const STORE_FACTOR: i32 = STORE_DATE + 1;
const STORE_TOTAL_FACTOR: i32 = STORE_FACTOR + 1;

// Columns in the stock splits view
const VIEW_DATE: i32 = 0;
const VIEW_FACTOR: i32 = 1;
const VIEW_TOTAL_FACTOR: i32 = 2;

const STOCK_SPLITS_WINDOW_HEIGHT: i32 = 80;
const STOCK_SPLITS_WINDOW_WIDTH: i32 = 400;

pub fn get_split_factor(split_guid: &str, globals:&Globals) -> f64 {
    prepare_statement!(GET_SPLIT_FACTOR_SQL, globals).query_row(params![split_guid], get_result!(f64))
                                                  .unwrap()
}

fn refresh_stock_splits_register(stock_splits_register: &StockSplitsRegister, globals:&Globals) {
    let view = &stock_splits_register.core.view;

    // Is there a current selection?
    if let Some((model, iter)) = view.get_selection().get_selected() {
        // Turn the iter into a path, since I don't think an iter can be valid after generating a new model
        if let Some(path) = model.get_path(&iter) {
            // Clear and repopulate the model
            populate_stock_splits_store(&stock_splits_register, globals);

            // Select something near the previously selected row, if there was one
            select_row(view, &path, &column_index_to_column(view, VIEW_FACTOR));
        }
    } else {
        // Generate a new model for the stock splits view and replace the old one
        populate_stock_splits_store(stock_splits_register, globals);

        // Select the first row
        select_first_row(view, &column_index_to_column(view, VIEW_FACTOR));
    }
}

fn factor_edited(path: &TreePath, new_factor: &str, stock_splits_register: &StockSplitsRegister,
                 globals: &Globals) {
    let store = &stock_splits_register.store;
    let stock_split_guid: String = get_string_column_via_path(store, path, STORE_GUID);
    if let Ok(factor) = new_factor.parse::<f64>() {
        // Write new value to store
        update_string_column_via_path(store, path, new_factor, STORE_FACTOR);

        prepare_statement!(UPDATE_SPLIT_FACTOR_SQL, globals).execute(params![factor,
                                                                          stock_split_guid])
                                                         .unwrap();
        refresh_stock_splits_register(&stock_splits_register, globals);
    } else {
        display_message_dialog("Invalid split factor", globals);
    }
}

fn new_stock_split(stock_splits_register: &StockSplitsRegister, globals:&Globals) {
    prepare_statement!(NEW_STOCK_SPLIT_SQL, globals)
        .execute(params![&(*stock_splits_register.commodity_guid)])
        .unwrap();
    refresh_stock_splits_register(stock_splits_register, globals);
}

fn delete_stock_split(stock_splits_register: &StockSplitsRegister, globals: &Globals) {
    let view = &stock_splits_register.core.view;
    if view.get_selection().count_selected_rows() == 1 {
        // We need the guid of the selected quote
        if let Some((model, iter)) = view.get_selection().get_selected() {
            let guid: String = model.get_value(&iter, STORE_GUID).get().unwrap();
            // Delete the quote
            prepare_statement!(DELETE_STOCK_SPLIT_SQL, globals).execute(params![guid])
                                                            .unwrap();
            refresh_stock_splits_register(stock_splits_register, globals);
        }
    } else {
        display_message_dialog("Improper selection", globals);
    }
}

fn display_calendar_for_stock_split(stock_splits_register: &StockSplitsRegister,
                                    globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&stock_splits_register.core, globals) {
        let current_timestamp: String = model.get_value(&iter, STORE_DATE).get().unwrap();
        let current_date: String = (&(current_timestamp.as_str())[0..DATE_SIZE]).to_string();

        if let Some(new_date) = display_calendar(&current_date, &stock_splits_register.core.window, globals)
        {
            let guid: String = stock_splits_register.store
                                                    .get_value(&iter, STORE_GUID)
                                                    .get()
                                                    .unwrap();
            date_edited(guid.as_str(),
                        prepare_statement!(STOCK_SPLIT_INCREMENT_DATE_SQL, globals),
                        prepare_statement!(STOCK_SPLIT_DATE_TO_FIRST_OF_MONTH_SQL, globals),
                        prepare_statement!(STOCK_SPLIT_DATE_TO_END_OF_MONTH_SQL, globals),
                        prepare_statement!(STOCK_SPLIT_DATE_TODAY_SQL, globals),
                        prepare_statement!(STOCK_SPLIT_DATE_TO_USER_ENTRY_SQL, globals),
                        &new_date,
                        &globals)
        }
        refresh_stock_splits_register(stock_splits_register, globals);
    }
}

fn populate_stock_splits_store(stock_splits_register: &StockSplitsRegister, globals:&Globals) {
    let store = &stock_splits_register.store;

    // Clear the store
    store.clear();

    // Fill the store
    let mut total_factor: f64 = 1.0;
    // Get stock split data
    let stmt = prepare_statement!(STOCK_SPLITS_REGISTER_SQL, globals);
    let stock_splits_iter = stmt
        .query_map(
            params![&(*stock_splits_register.commodity_guid)],
            get_result!(string_string_f64),
        )
        .unwrap();
    for wrapped_result in stock_splits_iter {
        let (guid, date, factor) = wrapped_result.unwrap();
        total_factor *= factor;
        // Append an empty row to the list store. Iter will point to the new row
        let iter = store.append();
        // Convert factor and total_factor to strings (6 decimal places), since we can't specify a data_func to do this
        // as was done in the C version
        let factor_string = format!("{:.*}", 6, factor);
        let total_factor_string = format!("{:.*}", 6, total_factor);
        // add data to the store
        store.set(&iter,
                  &[STORE_GUID as u32,
                    STORE_DATE as u32,
                    STORE_FACTOR as u32,
                    STORE_TOTAL_FACTOR as u32],
                  &[&guid, &date, &factor_string, &total_factor_string]);
    }
}

pub fn create_stock_splits_register(commodity_guid: &Rc<String>, fullname: String,
                                    globals: &Rc<Globals>) {
    // Create the descriptor
    let stock_splits_register =
        Rc::new(StockSplitsRegister { commodity_guid: commodity_guid.clone(),
                                      core: RegisterCore { view: TreeView::new(),
                                                           window:
                                                               Window::new(WindowType::Toplevel) },
                                      fullname,
                                      scrolled_window: ScrolledWindow::new(NONE_ADJUSTMENT,
                                                                           NONE_ADJUSTMENT),
                                      store: ListStore::new(&[Type::String, // Commodity guid
                                                              Type::String, // Split date
                                                              Type::String, // Split factor
                                                              Type::String  /* Total split factor */]) });

    // Unwrap optional entries used repeatedly below
    let view = &stock_splits_register.core.view;
    let window = &stock_splits_register.core.window;
    let store = &stock_splits_register.store;
    let scrolled_window = &stock_splits_register.scrolled_window;

    // Populate the model/store
    populate_stock_splits_store(&stock_splits_register, globals);
    // And point the view at it
    view.set_model(Some(store));

    // Column setup
    // Date
    {
        let renderer = CellRendererText::new();
        let closure_globals = globals.clone();
        let closure_stock_splits_register = stock_splits_register.clone();
        renderer.connect_edited(move |_, path, new_date| {
                    let quote_guid =
                        get_string_column_via_path(&closure_stock_splits_register.core
                                                                                 .view
                                                                                 .get_model()
                                                                                 .unwrap(),
                                                   &path,
                                                   STORE_GUID);
                    date_edited(&quote_guid,
                                prepare_statement!(STOCK_SPLIT_INCREMENT_DATE_SQL, closure_globals),
                                prepare_statement!(STOCK_SPLIT_DATE_TO_FIRST_OF_MONTH_SQL, closure_globals),
                                prepare_statement!(STOCK_SPLIT_DATE_TO_END_OF_MONTH_SQL, closure_globals),
                                prepare_statement!(STOCK_SPLIT_DATE_TODAY_SQL, closure_globals),
                                prepare_statement!(STOCK_SPLIT_DATE_TO_USER_ENTRY_SQL, closure_globals),
                                &new_date,
                                &closure_globals);
                    refresh_stock_splits_register(&closure_stock_splits_register, &closure_globals);
                });
        renderer.set_property_editable(true);
        // Add date column to the view
        let column: TreeViewColumn =
            create_tree_view_text_column(&renderer, "Split Date", STORE_DATE);
        view.insert_column(&column, VIEW_DATE);
    }
    // Factor
    {
        let renderer = CellRendererText::new();
        let globals = globals.clone();
        let closure_stock_splits_register = stock_splits_register.clone();
        renderer.connect_edited(move |_, path, new_factor| {
                    factor_edited(&path, &new_factor, &closure_stock_splits_register, &globals)
                });
        renderer.set_property_editable(true);

        let column: TreeViewColumn =
            create_tree_view_text_column(&renderer, "Split Factor", STORE_FACTOR);
        view.insert_column(&column, VIEW_FACTOR);
        // Right-justify the value column header
        column.set_alignment(1.0);
        // Make renderer right-justify the data
        renderer.set_alignment(1.0, 0.5);
    }
    // Total Factor
    {
        let renderer = CellRendererText::new();
        let column: TreeViewColumn =
            create_tree_view_text_column(&renderer, "Total Split Factor", STORE_TOTAL_FACTOR);
        view.insert_column(&column, VIEW_TOTAL_FACTOR);
        // Right-justify the value column header
        column.set_alignment(1.0);
        // Make renderer right-justify the data
        renderer.set_alignment(1.0, 0.5);
    }

    // Grid lines for readability
    view.set_grid_lines(Both);

    // Set geometry hints
    window.get_preferred_width(); // Do these two calls to avoid annoying warnings from gtk
    window.get_preferred_height();
    window.set_default_size(STOCK_SPLITS_WINDOW_WIDTH, STOCK_SPLITS_WINDOW_HEIGHT);

    scrolled_window.add(view);
    window.add(scrolled_window);

    // Set window title to commodity name
    window.set_title(&stock_splits_register.fullname);

    // Set up to handle mouse button press events
    // Build the top-level popup menu
    let stock_splits_menu = Menu::new();
    {
        let stock_splits_menu_item = MenuItem::new_with_label("New stock split (ctrl-n)");
        let closure_globals = globals.clone();
        let closure_stock_splits_register = stock_splits_register.clone();
        stock_splits_menu_item.connect_activate(move |_stock_splits_menu_item: &MenuItem| {
                                  new_stock_split(&closure_stock_splits_register, &closure_globals);
                              });
        stock_splits_menu.append(&stock_splits_menu_item);
    }
    {
        let stock_splits_menu_item =
            MenuItem::new_with_label("Delete selected stock split (ctrl-shift-d)");
        let closure_globals = globals.clone();
        let closure_stock_splits_register = stock_splits_register.clone();
        stock_splits_menu_item.connect_activate(move |_stock_splits_menu_item: &MenuItem| {
                                  delete_stock_split(&closure_stock_splits_register,
                                                     &closure_globals);
                              });
        stock_splits_menu.append(&stock_splits_menu_item);
    }
    {
        let stock_splits_menu_item =
            MenuItem::new_with_label("Display calendar for selected stock split (Ctrl-a)");
        let closure_globals = globals.clone();
        let closure_stock_splits_register = stock_splits_register.clone();
        stock_splits_menu_item.connect_activate(move |_stock_splits_menu_item: &MenuItem| {
                                  display_calendar_for_stock_split(&closure_stock_splits_register,
                                                                   &closure_globals);
                              });
        stock_splits_menu.append(&stock_splits_menu_item);
    }

    view.connect_button_press_event(move |_view: &TreeView, event_button: &EventButton| {
            // single click and right button pressed?
            if (event_button.get_event_type() == ButtonPress) && (event_button.get_button() == 3) {
                stock_splits_menu.show_all();
                stock_splits_menu.popup_easy(3, event_button.get_time());
                Inhibit(true) // we handled this
            } else {
                Inhibit(false) // we did not handle this
            }
        });

    // Connect to signal for key press events
    let globals_key_press_event = globals.clone();
    let stock_splits_register_key_press_event = stock_splits_register.clone();
    view.connect_key_press_event(move |_stock_splits_view: &TreeView, event_key: &EventKey| {
            let masked_state: u32 =
                event_key.get_state().bits() & globals_key_press_event.modifiers.bits();
            // Ctrl key pressed?
            if masked_state == ModifierType::CONTROL_MASK.bits() {
                match event_key.get_keyval() {
                    key::n => {
                        new_stock_split(&stock_splits_register_key_press_event, &globals_key_press_event);
                        Inhibit(true)
                    }
                    key::d => {
                        delete_stock_split(&stock_splits_register_key_press_event,
                                           &globals_key_press_event);
                        Inhibit(true)
                    }
                    key::a => {
                        display_calendar_for_stock_split(&stock_splits_register_key_press_event,
                                                         &globals_key_press_event);
                        Inhibit(true)
                    }
                    // Indicate we didn't handle the event
                    _ => Inhibit(false),
                }
            } else if masked_state
                      == (ModifierType::CONTROL_MASK.bits() | ModifierType::SHIFT_MASK.bits())
            {
                match event_key.get_keyval() {
                    key::D => {
                        delete_stock_split(&stock_splits_register_key_press_event,
                                           &globals_key_press_event);
                        Inhibit(true)
                    }
                    // Indicate we didn't handle the event
                    _ => Inhibit(false),
                }
            } else {
                // We didn't handle the event
                Inhibit(false)
            }
        });

    // Set the view's selection mode
    view.get_selection().set_mode(Browse);

    window.show_all();
}
