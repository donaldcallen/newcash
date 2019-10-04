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
use constants::{CommodityRegister, Globals, RegisterCore};
use gdk::enums::key;
use gdk::EventType::ButtonPress;
use gdk::{EventButton, EventKey, ModifierType};
use gtk::SelectionMode::Browse;
use gtk::TreeViewGridLines::Both;
use gtk::{
    CellRendererText, CellRendererTextExt, ContainerExt, GtkListStoreExt, GtkListStoreExtManual,
    GtkMenuExtManual, GtkMenuItemExt, GtkWindowExt, Inhibit, ListStore, Menu, MenuItem,
    MenuShellExt, ScrolledWindow, TreeModelExt, TreePath, TreeSelectionExt, TreeView,
    TreeViewColumn, TreeViewColumnExt, TreeViewExt, Type, WidgetExt, Window, WindowType,
    NONE_ADJUSTMENT,
};
use queries::{
    DELETE_QUOTE_SQL, NEW_QUOTE_SQL, PRICES_SQL, QUOTE_INCREMENT_TIMESTAMP_SQL,
    QUOTE_TIMESTAMP_TODAY_SQL, QUOTE_TIMESTAMP_TO_END_OF_MONTH_SQL,
    QUOTE_TIMESTAMP_TO_FIRST_OF_MONTH_SQL, QUOTE_TIMESTAMP_TO_USER_ENTRY_SQL,
    QUOTE_UPDATE_VALUE_SQL,
};
use std::rc::Rc;
use utilities::{
    column_index_to_column, create_tree_view_text_column, date_edited, display_message_dialog,
    get_selection_info, get_string_column_via_path, select_first_row,
    update_string_column_via_path,
};

// Columns placed in the commodity register store
const STORE_GUID: i32 = 0;
const STORE_TIMESTAMP: i32 = STORE_GUID + 1;
const STORE_PRICE: i32 = STORE_TIMESTAMP + 1;

// Columns in the commodity register view
const VIEW_TIMESTAMP: i32 = 0;
const VIEW_PRICE: i32 = VIEW_TIMESTAMP + 1;

const COMMODITY_WINDOW_HEIGHT: i32 = 300;
const COMMODITY_WINDOW_WIDTH: i32 = 300;

fn refresh_commodity_register(commodity_register: &CommodityRegister, globals: &Globals) {
    let view = &commodity_register.core.view;
    // Clear the store
    commodity_register.store.clear();
    populate_commodity_register_store(commodity_register, globals);

    select_first_row(view, &column_index_to_column(view, VIEW_PRICE));
}

fn delete_quote(commodity_register: &CommodityRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&commodity_register.core, globals) {
        let quote_guid: String = model.get_value(&iter, STORE_GUID).get().unwrap();
        prepare_statement!(DELETE_QUOTE_SQL, globals).execute(params![quote_guid])
                                                  .unwrap();
        // And refresh the commodity register, so we can see the change
        refresh_commodity_register(&commodity_register, globals);
    }
}

// Called when calendar requested for transaction
fn display_calendar_for_quote(commodity_register: &CommodityRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&commodity_register.core, globals) {
        let current_date: String = model.get_value(&iter, STORE_TIMESTAMP).get().unwrap();
        if let Some(new_date) = display_calendar(&current_date, &commodity_register.core.window, globals) {
            let quote_guid: String = model.get_value(&iter, STORE_GUID).get().unwrap();
            date_edited(&quote_guid,
                        prepare_statement!(QUOTE_INCREMENT_TIMESTAMP_SQL, globals),
                        prepare_statement!(QUOTE_TIMESTAMP_TO_FIRST_OF_MONTH_SQL, globals),
                        prepare_statement!(QUOTE_TIMESTAMP_TO_END_OF_MONTH_SQL, globals),
                        prepare_statement!(QUOTE_TIMESTAMP_TODAY_SQL, globals),
                        prepare_statement!(QUOTE_TIMESTAMP_TO_USER_ENTRY_SQL, globals),
                        &new_date,
                        &globals);
            refresh_commodity_register(commodity_register, globals);
        }
    }
}

fn new_quote(commodity_register: &CommodityRegister, globals: &Globals) {
    prepare_statement!(NEW_QUOTE_SQL, globals).execute(params![commodity_register.guid])
                                           .unwrap();
    refresh_commodity_register(&commodity_register, globals);
}

// Called when value field is edited
fn value_field_edited(path: &TreePath, store_index: i32, new_value: &str,
                      commodity_register: &CommodityRegister, globals: &Globals) {
    let store = &commodity_register.store;
    let commodity_guid: String = get_string_column_via_path(store, path, STORE_GUID);

    // Check to be sure the new value the user typed is numeric
    if let Ok(parsed_value) = new_value.parse::<f64>() {
        // Update the database
        prepare_statement!(QUOTE_UPDATE_VALUE_SQL, globals).execute(params![parsed_value,
                                                                         commodity_guid])
                                                        .unwrap();

        // Write new value to store
        update_string_column_via_path(store, path, new_value, store_index);
    } else {
        display_message_dialog("You have entered a non-numeric value in the price field of a \
                                quote.",
                               globals);
    }
}

fn populate_commodity_register_store(commodity_register: &CommodityRegister, globals: &Globals) {
    let store = &commodity_register.store;
    // Set up the query that fetches price data to produce the register.
    let stmt = prepare_statement!(PRICES_SQL, globals);
    let prices_iter =
        stmt.query_map(params![commodity_register.guid],
                                                       get_result!(string_string_f64))
                                            .unwrap();
    for wrapped_result in prices_iter {
        let (guid, timestamp, price) = wrapped_result.unwrap();
        // Append an empty row to the list store. Iter will point to the new row
        let iter = store.append();
        let price_string = format!("{:.*}", 4, price);
        // add data
        store.set(&iter,
                  &[STORE_GUID as u32,
                    STORE_TIMESTAMP as u32,
                    STORE_PRICE as u32],
                  &[&guid, &timestamp, &price_string]);
    }
}

pub fn create_commodity_register(commodity_guid: String, commodity_name: &str,
                                 globals: &Rc<Globals>) {
    // Build the account register
    let commodity_register =
        Rc::new(CommodityRegister { core: RegisterCore { view: TreeView::new(),
                                                         window:
                                                             Window::new(WindowType::Toplevel) },
                                    guid: commodity_guid,
                                    scrolled_window: ScrolledWindow::new(NONE_ADJUSTMENT,
                                                                         NONE_ADJUSTMENT),
                                    store: ListStore::new(&[Type::String, // guid
                                                            Type::String, // symbol/mnemonic
                                                            Type::String, // timestamp
                                                            Type::String  /* price */]) });

    // Unwrap optional entries used repeatedly below
    let view = &commodity_register.core.view;
    let window = &commodity_register.core.window;
    let store = &commodity_register.store;
    let scrolled_window = &commodity_register.scrolled_window;

    // Populate the model/store
    populate_commodity_register_store(&commodity_register, globals);

    // Column setup
    // Timestamp
    {
        let renderer = CellRendererText::new();
        let closure_globals = globals.clone();
        let closure_commodity_register = commodity_register.clone();
        renderer.connect_edited(move |_, path, new_timestamp| {
                    let guid: String =
                        get_string_column_via_path(&closure_commodity_register.core
                                                                              .view
                                                                              .get_model()
                                                                              .unwrap(),
                                                   &path,
                                                   STORE_GUID);
                    date_edited(&guid,
                                prepare_statement!(QUOTE_INCREMENT_TIMESTAMP_SQL, closure_globals),
                                prepare_statement!(QUOTE_TIMESTAMP_TO_FIRST_OF_MONTH_SQL, closure_globals),
                                prepare_statement!(QUOTE_TIMESTAMP_TO_END_OF_MONTH_SQL, closure_globals),
                                prepare_statement!(QUOTE_TIMESTAMP_TODAY_SQL, closure_globals),
                                prepare_statement!(QUOTE_TIMESTAMP_TO_USER_ENTRY_SQL, closure_globals),
                                &new_timestamp,
                                &closure_globals);
                    refresh_commodity_register(&closure_commodity_register, &closure_globals);
                });
        // Add column to the view
        let column: TreeViewColumn =
            create_tree_view_text_column(&renderer, "Time-stamp", STORE_TIMESTAMP);
        view.insert_column(&column, VIEW_TIMESTAMP);
        column.set_expand(true);
    }
    // Price
    {
        let renderer = CellRendererText::new();
        let closure_globals = globals.clone();
        let closure_commodity_register = commodity_register.clone();
        renderer.connect_edited(move |_, path, new_price| {
                    value_field_edited(&path,
                                       STORE_PRICE,
                                       new_price,
                                       &closure_commodity_register,
                                       &closure_globals);
                });
        renderer.set_property_editable(true);
        // Add column to the view
        let column: TreeViewColumn = create_tree_view_text_column(&renderer, "Price", STORE_PRICE);
        view.insert_column(&column, VIEW_PRICE);
        column.set_resizable(true);
        column.set_expand(true);
    }

    // Set up to handle mouse button press events
    // Build the top-level popup menu
    let commodity_register_menu = Menu::new();
    {
        let commodity_register_menu_item = MenuItem::new_with_label("New quote (ctrl-n)");
        let closure_commodity_register = commodity_register.clone();
        let closure_globals = globals.clone();
        commodity_register_menu_item.connect_activate(
            move |_commodity_register_menu_item: &MenuItem| {
                new_quote(&closure_commodity_register, &closure_globals);
            },
        );
        commodity_register_menu.append(&commodity_register_menu_item);
    }

    {
        let commodity_register_menu_item =
            MenuItem::new_with_label("Delete selected quote (ctrl-shift-d)");
        let closure_globals = globals.clone();
        let closure_commodity_register = commodity_register.clone();
        commodity_register_menu_item.connect_activate(
            move |_commodity_register_menu_item: &MenuItem| {
                delete_quote(&closure_commodity_register, &closure_globals);
            },
        );
        commodity_register_menu.append(&commodity_register_menu_item);
    }

    {
        let commodity_register_menu_item =
            MenuItem::new_with_label("Display calendar for selected transaction (Ctrl-a)");
        let closure_globals = globals.clone();
        let closure_commodity_register = commodity_register.clone();
        commodity_register_menu_item.connect_activate(
            move |_commodity_register_menu_item: &MenuItem| {
                display_calendar_for_quote(&closure_commodity_register, &closure_globals);
            },
        );
        commodity_register_menu.append(&commodity_register_menu_item);
    }

    view.connect_button_press_event(move |_view: &TreeView, event_button: &EventButton| {
            // single click and right button pressed?
            if (event_button.get_event_type() == ButtonPress) && (event_button.get_button() == 3) {
                commodity_register_menu.show_all();
                commodity_register_menu.popup_easy(3, event_button.get_time());
                Inhibit(true) // we handled this
            } else {
                Inhibit(false) // we did not handle this
            }
        });

    // Connect to signal for key press events
    let globals_key_press_event = globals.clone();
    let commodity_register_key_press_event = commodity_register.clone();
    view.connect_key_press_event(move |_accounts_view: &TreeView, event_key: &EventKey| {
            let masked_state: u32 =
                event_key.get_state().bits() & globals_key_press_event.modifiers.bits();
            // Ctrl key pressed?
            if masked_state == ModifierType::CONTROL_MASK.bits() {
                match event_key.get_keyval() {
                    key::n => {
                        new_quote(&commodity_register_key_press_event, &globals_key_press_event);
                        Inhibit(true)
                    }
                    key::a => {
                        display_calendar_for_quote(&commodity_register_key_press_event,
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
                        delete_quote(&commodity_register_key_press_event,
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

    // Hook up store to the view
    view.set_model(Some(store));

    // Grid lines for readability
    view.set_grid_lines(Both);

    scrolled_window.add(view);
    window.add(scrolled_window);

    // Set the view's selection mode
    view.get_selection().set_mode(Browse);

    // Set window title to account name
    window.set_title(commodity_name);

    // Set window size
    window.get_preferred_width(); // Do these two calls to avoid annoying warnings from gtk
    window.get_preferred_height();
    window.resize(COMMODITY_WINDOW_WIDTH, COMMODITY_WINDOW_HEIGHT);

    window.show_all();
}
