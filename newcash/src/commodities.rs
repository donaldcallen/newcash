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

use commodity::create_commodity_register;
use constants::{CommoditiesRegister, FindCommand, FindParameters, Globals, RegisterCore};
use gdk::enums::key;
use gdk::Atom;
use gdk::EventType::ButtonPress;
use gdk::{EventButton, EventKey, ModifierType};
use glib::types::Type;
use gtk::prelude::{GtkListStoreExtManual, GtkMenuExtManual};
use gtk::SelectionMode::Browse;
use gtk::TreeViewGridLines::Both;
use gtk::{
    CellRendererText, CellRendererTextExt, CellRendererToggle, CellRendererToggleExt, Clipboard,
    ContainerExt, GtkListStoreExt, GtkMenuItemExt, GtkWindowExt, Inhibit, ListStore, Menu,
    MenuItem, MenuShellExt, ScrolledWindow, TreeModelExt, TreePath, TreeSelectionExt, TreeView,
    TreeViewColumn, TreeViewColumnExt, TreeViewExt, WidgetExt, Window, WindowType, NONE_ADJUSTMENT,
};
use queries::{
    ACCOUNTS_LINKED_TO_COMMODITY_SQL, CHECK_INUSE_COMMODITY_SQL, COMMODITIES_SQL,
    DELETE_COMMODITY_SQL, DUPLICATE_COMMODITY_SQL, LATEST_QUOTE_TIMESTAMP_SQL, NEW_COMMODITY_SQL,
    TOGGLE_COMMODITY_MM_FLAG_SQL,
};
use rusqlite::params;
use rust_library::constants::COMMODITY_FLAG_MONEY_MARKET_FUND;
use rust_library::guid_to_path;
use rust_library::queries::{GUID_TO_PATH_SQL, NEW_UUID_SQL};
use std::cell::RefCell;
use std::rc::Rc;
use stock_splits::create_stock_splits_register;
use utilities::{
    column_index_to_column, create_tree_view_text_column, create_tree_view_toggle_column,
    display_message_dialog, find, get_selection_info, get_string_column_via_path, select_last_row,
    select_row, select_row_by_guid, update_boolean_column_via_path, update_string_column_via_path,
};

// Columns returned by the commodities register query

const QUERY_GUID: usize = 0;
const QUERY_SYMBOL: usize = QUERY_GUID + 1;
const QUERY_NAME: usize = QUERY_SYMBOL + 1;
const QUERY_CUSIP: usize = QUERY_NAME + 1;
const QUERY_FLAGS: usize = QUERY_CUSIP + 1;

// Columns placed in the commodities register store
const STORE_GUID: i32 = 0;
const STORE_SYMBOL: i32 = STORE_GUID + 1;
const STORE_NAME: i32 = STORE_SYMBOL + 1;
const STORE_CUSIP: i32 = STORE_NAME + 1;
const STORE_MM: i32 = STORE_CUSIP + 1;

// Columns in the commodities register view
const VIEW_SYMBOL: i32 = 0;
const VIEW_NAME: i32 = VIEW_SYMBOL + 1;
const VIEW_CUSIP: i32 = VIEW_NAME + 1;
const VIEW_MM: i32 = VIEW_CUSIP + 1;

// Ths must be kept in sync with the columns actually in the store
const STORE_COLUMN_NAMES: [&str; 4] = ["Symbol", "Name", "Cusip", "Money Market"];
const STORE_COLUMN_INDICES: [i32; 4] = [STORE_SYMBOL, STORE_NAME, STORE_CUSIP, STORE_MM];
const STORE_COLUMN_TYPES: [Type; 4] = [Type::String, Type::String, Type::String, Type::Bool];

const COMMODITIES_WINDOW_HEIGHT: i32 = 400;
const COMMODITIES_WINDOW_WIDTH: i32 = 800;

fn display_stock_splits_register(
    commodities_register: &CommoditiesRegister, globals: &Rc<Globals>,
) {
    // To find the stock splits, we need the commodity guid
    if let Some((model, iter)) = get_selection_info(&commodities_register.core, &globals) {
        let commodity_guid: Rc<String> =
            Rc::new(model.get_value(&iter, STORE_GUID).get().unwrap().unwrap());
        let commodity_name: String = model.get_value(&iter, STORE_NAME).get().unwrap().unwrap();
        create_stock_splits_register(&commodity_guid, commodity_name, globals);
    }
}

fn refresh_commodities_register(
    commodities_register: &CommoditiesRegister, new_commodity_guid: Option<&String>,
    globals: &Globals,
) {
    let path: Option<TreePath> =
        if let Some((model, iter)) = get_selection_info(&commodities_register.core, globals) {
            model.get_path(&iter)
        } else {
            None
        };

    let view = &commodities_register.core.view;

    // Clear the store
    commodities_register.store.clear();
    populate_commodities_register_store(commodities_register, globals);

    if let Some(guid) = new_commodity_guid {
        select_row_by_guid(view, &guid, STORE_GUID, &column_index_to_column(view, VIEW_NAME));
    } else if let Some(p) = path {
        // Select something near the previously selected row, if there was one, or select the last row
        select_row(view, &p, &column_index_to_column(view, VIEW_NAME));
    } else {
        select_last_row(view, &column_index_to_column(view, VIEW_NAME));
    }
}

fn display_linked_accounts(commodity_guid: String, to_clipboard: bool, globals: &Globals) {
    let mut account_paths: String = "".to_string();
    let stmt = prepare_statement!(ACCOUNTS_LINKED_TO_COMMODITY_SQL, globals);
    let iter = stmt.query_map(params![commodity_guid], get_result!(string)).unwrap();
    for wrapped_result in iter {
        let account_guid: String = wrapped_result.unwrap();
        let account_path =
            guid_to_path(prepare_statement!(GUID_TO_PATH_SQL, globals), &account_guid);
        account_paths = format!("{}{}\n", account_paths, account_path);
    }
    if account_paths.is_empty() {
        account_paths = "None".to_string();
    }
    if to_clipboard {
        Clipboard::get(&Atom::intern("CLIPBOARD")).set_text(account_paths.as_str());
    } else {
        display_message_dialog(account_paths.as_str(), globals);
    };
}

fn display_most_recent_quote_timestamp(globals: &Globals) {
    let most_recent_quote_date = prepare_statement!(LATEST_QUOTE_TIMESTAMP_SQL, globals)
        .query_row(params![], get_result!(string))
        .unwrap();
    display_message_dialog(
        &format!("Quotes are up-to-date as of {}", most_recent_quote_date),
        globals,
    );
}

fn delete_commodity(commodities_register: &CommoditiesRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&commodities_register.core, globals) {
        let commodity_guid: String = model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
        // are there any accounts still using this commodity?
        let n_users = prepare_statement!(CHECK_INUSE_COMMODITY_SQL, globals)
            .query_row(params![commodity_guid], get_result!(i32))
            .unwrap();

        // Make sure no accounts are using the commodity
        if n_users == 0 {
            prepare_statement!(DELETE_COMMODITY_SQL, globals)
                .execute(params![commodity_guid])
                .unwrap();
            // And refresh the commodities register, so we can see the change
            refresh_commodities_register(&commodities_register, None, globals);
        } else {
            display_message_dialog(
                "There are accounts using this commodity; it cannot be \
                                    deleted.
To delete this commodity, you must either delete those accounts first 
or assign other commodities to them.",
                globals,
            );
        }
    }
}

fn duplicate_commodity(commodities_register: &CommoditiesRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&commodities_register.core, globals) {
        let new_commodity_guid = prepare_statement!(NEW_UUID_SQL, globals)
            .query_row(params![], get_result!(string))
            .unwrap();
        let source_commodity_guid: String =
            model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
        // Create the new commodity with new guid
        prepare_statement!(DUPLICATE_COMMODITY_SQL, globals)
            .execute(params![new_commodity_guid, source_commodity_guid])
            .unwrap();
        refresh_commodities_register(&commodities_register, Some(&new_commodity_guid), globals);
    }
}

fn new_commodity(commodities_register: &CommoditiesRegister, globals: &Globals) {
    let new_commodity_guid = prepare_statement!(NEW_UUID_SQL, globals)
        .query_row(params![], get_result!(string))
        .unwrap();
    prepare_statement!(NEW_COMMODITY_SQL, globals).execute(params![new_commodity_guid]).unwrap();
    refresh_commodities_register(&commodities_register, Some(&new_commodity_guid), globals);
}

// Called when symbol is edited
fn symbol_edited(
    path: &TreePath, new_symbol: &str, commodities_register: &CommoditiesRegister,
    globals: &Globals,
) {
    let store = &commodities_register.store;
    let commodity_guid: String = get_string_column_via_path(store, path, STORE_GUID);

    // Update the database
    prepare_statement!("update commodities set mnemonic = ?1 where guid = ?2", globals)
        .execute(params![new_symbol.to_string(), commodity_guid])
        .unwrap();

    // Write new value to store
    update_string_column_via_path(store, path, new_symbol, STORE_SYMBOL);
}

// Called when name is edited
fn name_edited(
    path: &TreePath, new_name: &str, commodities_register: &CommoditiesRegister, globals: &Globals,
) {
    let store = &commodities_register.store;
    let commodity_guid: String = get_string_column_via_path(store, path, STORE_GUID);

    // Update the database
    prepare_statement!("update commodities set fullname = ?1 where guid = ?2", globals)
        .execute(params![new_name.to_string(), commodity_guid])
        .unwrap();

    // Write new value to store
    update_string_column_via_path(store, path, new_name, STORE_NAME);
}

// Called when cusip is edited
fn cusip_edited(
    path: &TreePath, new_cusip: &str, commodities_register: &CommoditiesRegister, globals: &Globals,
) {
    let store = &commodities_register.store;
    let commodity_guid: String = get_string_column_via_path(store, path, STORE_GUID);

    // Update the database
    prepare_statement!("update commodities set cusip = ?1 where guid = ?2", globals)
        .execute(params![new_cusip.to_string(), commodity_guid])
        .unwrap();

    // Write new value to store
    update_string_column_via_path(store, path, new_cusip, STORE_CUSIP);
}

// Called when 'toggled' is signalled for MM column
fn mm_toggled(
    renderer: &CellRendererToggle, path: &TreePath, commodities_register: &CommoditiesRegister,
    globals: &Globals,
) {
    let store = &commodities_register.store;
    let commodity_guid: String = get_string_column_via_path(store, path, STORE_GUID);

    // Update the database
    prepare_statement!(TOGGLE_COMMODITY_MM_FLAG_SQL, globals)
        .execute(params![commodity_guid])
        .unwrap();

    // Update the model and view
    update_boolean_column_via_path(store, path, !renderer.get_active(), STORE_MM);
}

fn populate_commodities_register_store(
    commodities_register: &CommoditiesRegister, globals: &Globals,
) {
    let store = &commodities_register.store;
    // Set up the query that fetches commodity data to produce the register.
    let stmt = prepare_statement!(COMMODITIES_SQL, globals);
    let commodities_iter = stmt
        .query_map(
            params![],
            |row| -> Result<(String, String, String, String, i32), rusqlite::Error> {
                Ok((
                    row.get(QUERY_GUID).unwrap(),
                    row.get(QUERY_SYMBOL).unwrap(),
                    row.get(QUERY_NAME).unwrap(),
                    row.get(QUERY_CUSIP).unwrap(),
                    row.get(QUERY_FLAGS).unwrap(),
                ))
            },
        )
        .unwrap();
    for wrapped_result in commodities_iter {
        let (guid, symbol, name, cusip, flags) = wrapped_result.unwrap();
        // Append an empty row to the list store. Iter will point to the new row
        let iter = store.append();
        let money_market_p: bool = (flags & COMMODITY_FLAG_MONEY_MARKET_FUND) != 0;

        // add data
        store.set(
            &iter,
            &[
                STORE_GUID as u32,
                STORE_SYMBOL as u32,
                STORE_NAME as u32,
                STORE_CUSIP as u32,
                STORE_MM as u32,
            ],
            &[&guid, &symbol, &name, &cusip, &money_market_p],
        );
    }
}

fn create_commodities_store() -> ListStore {
    ListStore::new(&[
        Type::String, // guid
        Type::String, // symbol/mnemonic
        Type::String, // name
        Type::String, // cusip
        Type::Bool,   /* MM */
    ])
}

pub fn create_commodities_register(globals: &Rc<Globals>) {
    // Build the account register
    let commodities_register = Rc::new(CommoditiesRegister {
        core: RegisterCore {
            view: TreeView::new(),
            window: Window::new(WindowType::Toplevel),
        },
        find_parameters: RefCell::new(FindParameters {
            column_index: None,
            path: None,
            regex: None,
            column_type: None,
            column_names: &STORE_COLUMN_NAMES,
            column_indices: &STORE_COLUMN_INDICES,
            column_types: &STORE_COLUMN_TYPES,
            default_store_column: STORE_NAME,
            default_view_column: VIEW_NAME as u32,
        }),
        scrolled_window: ScrolledWindow::new(NONE_ADJUSTMENT, NONE_ADJUSTMENT),
        store: create_commodities_store(),
    });

    // Unwrap optional entries used repeatedly below
    let view = &commodities_register.core.view;
    let window = &commodities_register.core.window;
    let store = &commodities_register.store;
    let scrolled_window = &commodities_register.scrolled_window;

    // Populate the model/store
    populate_commodities_register_store(&commodities_register, &globals);

    // Column setup
    // Mnemonic (symbol)
    {
        let renderer = CellRendererText::new();
        let closure_commodities_register = commodities_register.clone();
        let closure_globals = globals.clone();
        renderer.connect_edited(move |_, path, new_symbol| {
            symbol_edited(&path, new_symbol, &closure_commodities_register, &closure_globals);
        });
        renderer.set_property_editable(true);
        // Add column to the view
        let column: TreeViewColumn =
            create_tree_view_text_column(&renderer, "Symbol", STORE_SYMBOL);
        view.insert_column(&column, VIEW_SYMBOL);
        column.set_expand(true);
    }
    // Name
    {
        let renderer = CellRendererText::new();
        let closure_commodities_register = commodities_register.clone();
        let closure_globals = globals.clone();
        renderer.connect_edited(move |_, path, new_name| {
            name_edited(&path, new_name, &closure_commodities_register, &closure_globals);
        });
        renderer.set_property_editable(true);
        // Add column to the view
        let column: TreeViewColumn = create_tree_view_text_column(&renderer, "Name", STORE_NAME);
        view.insert_column(&column, VIEW_NAME);
        column.set_resizable(true);
        column.set_expand(true);
    }

    // Cusip
    {
        let renderer = CellRendererText::new();
        let closure_commodities_register = commodities_register.clone();
        let closure_globals = globals.clone();
        renderer.connect_edited(move |_, path, new_cusip| {
            cusip_edited(&path, new_cusip, &closure_commodities_register, &closure_globals);
        });
        renderer.set_property_editable(true);
        // Add column to the view
        let column: TreeViewColumn = create_tree_view_text_column(&renderer, "Cusip", STORE_CUSIP);
        view.insert_column(&column, VIEW_CUSIP);
        column.set_expand(true);
    }

    // MM (money market fund flag)
    {
        let renderer = CellRendererToggle::new();
        let closure_commodities_register = commodities_register.clone();
        let closure_globals = globals.clone();
        renderer.connect_toggled(move |closure_renderer, path| {
            mm_toggled(closure_renderer, &path, &closure_commodities_register, &closure_globals);
        });
        renderer.set_activatable(true);
        // Add column to the view
        let column: TreeViewColumn = create_tree_view_toggle_column(&renderer, "MM", STORE_MM);
        view.insert_column(&column, VIEW_MM);
    }

    // Set up to handle mouse button press events
    // Build the top-level popup menu
    let commodities_register_menu = Menu::new();
    {
        let commodities_register_menu_item = MenuItem::new_with_label("New commodity (Ctrl-n)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                new_commodity(&closure_commodities_register, &closure_globals);
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Duplicate selected commodity (Ctrl-d)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                duplicate_commodity(&closure_commodities_register, &closure_globals);
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Delete selected commodity (Ctrl-Shift-d)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                delete_commodity(&closure_commodities_register, &closure_globals);
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item = MenuItem::new_with_label("Find commodity (Ctrl-f)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                find(
                    &FindCommand::FindForward,
                    &closure_commodities_register.find_parameters,
                    &closure_commodities_register.core,
                    &closure_globals,
                );
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Find next commodity (Ctrl-g)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                find(
                    &FindCommand::FindNextForward,
                    &closure_commodities_register.find_parameters,
                    &closure_commodities_register.core,
                    &closure_globals,
                );
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Display commodity register (Ctrl-o)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                if let Some((model, iter)) =
                    get_selection_info(&closure_commodities_register.core, &closure_globals)
                {
                    let commodity_guid: String =
                        model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
                    let commodity_name: String =
                        model.get_value(&iter, STORE_NAME).get().unwrap().unwrap();
                    create_commodity_register(commodity_guid, &commodity_name, &closure_globals);
                    Inhibit(true);
                } else {
                    Inhibit(false);
                }
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Display linked accounts (Ctrl-a)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                if let Some((model, iter)) =
                    get_selection_info(&closure_commodities_register.core, &closure_globals)
                {
                    let commodity_guid: String =
                        model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
                    display_linked_accounts(commodity_guid, false, &closure_globals);
                    Inhibit(true);
                } else {
                    Inhibit(false);
                }
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Linked accounts to system clipboard (Ctrl-Shift-a)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                if let Some((model, iter)) =
                    get_selection_info(&closure_commodities_register.core, &closure_globals)
                {
                    let commodity_guid: String =
                        model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
                    display_linked_accounts(commodity_guid, true, &closure_globals);
                    Inhibit(true);
                } else {
                    Inhibit(false);
                }
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Display most recent quote timestamp (Ctrl-t)");
        let closure_globals = globals.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                display_most_recent_quote_timestamp(&closure_globals);
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }
    {
        let commodities_register_menu_item =
            MenuItem::new_with_label("Display stock splits register (Ctrl-s)");
        let closure_globals = globals.clone();
        let closure_commodities_register = commodities_register.clone();
        commodities_register_menu_item.connect_activate(
            move |_commodities_register_menu_item: &MenuItem| {
                display_stock_splits_register(&closure_commodities_register, &closure_globals);
            },
        );
        commodities_register_menu.append(&commodities_register_menu_item);
    }

    view.connect_button_press_event(move |_view: &TreeView, event_button: &EventButton| {
        // single click and right button pressed?
        if (event_button.get_event_type() == ButtonPress) && (event_button.get_button() == 3) {
            commodities_register_menu.show_all();
            commodities_register_menu.popup_easy(3, event_button.get_time());
            Inhibit(true) // we handled this
        } else {
            Inhibit(false) // we did not handle this
        }
    });

    // Connect to signal for key press events
    let globals_key_press_event = globals.clone();
    let commodities_register_key_press_event = commodities_register.clone();
    view.connect_key_press_event(move |_accounts_view: &TreeView, event_key: &EventKey| {
        let masked_state: u32 =
            event_key.get_state().bits() & globals_key_press_event.modifiers.bits();
        // Ctrl key pressed?
        if masked_state == ModifierType::CONTROL_MASK.bits() {
            match event_key.get_keyval() {
                key::n => {
                    new_commodity(&commodities_register_key_press_event, &globals_key_press_event);
                    Inhibit(true)
                }
                key::f => {
                    find(
                        &FindCommand::FindForward,
                        &commodities_register_key_press_event.find_parameters,
                        &commodities_register_key_press_event.core,
                        &globals_key_press_event,
                    );
                    Inhibit(true)
                }
                key::g => {
                    find(
                        &FindCommand::FindNextForward,
                        &commodities_register_key_press_event.find_parameters,
                        &commodities_register_key_press_event.core,
                        &globals_key_press_event,
                    );
                    Inhibit(true)
                }
                key::d => {
                    duplicate_commodity(
                        &commodities_register_key_press_event,
                        &globals_key_press_event,
                    );
                    Inhibit(true)
                }
                key::o => {
                    if let Some((model, iter)) = get_selection_info(
                        &commodities_register_key_press_event.core,
                        &globals_key_press_event,
                    ) {
                        let commodity_guid: String =
                            model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
                        let commodity_name: String =
                            model.get_value(&iter, STORE_NAME).get().unwrap().unwrap();
                        create_commodity_register(
                            commodity_guid,
                            &commodity_name,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    } else {
                        Inhibit(false)
                    }
                }
                key::a => {
                    if let Some((model, iter)) = get_selection_info(
                        &commodities_register_key_press_event.core,
                        &globals_key_press_event,
                    ) {
                        let commodity_guid: String =
                            model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
                        display_linked_accounts(commodity_guid, false, &globals_key_press_event);
                        Inhibit(true)
                    } else {
                        Inhibit(false)
                    }
                }
                key::t => {
                    display_most_recent_quote_timestamp(&globals_key_press_event);
                    Inhibit(true)
                }
                key::s => {
                    display_stock_splits_register(
                        &commodities_register_key_press_event,
                        &globals_key_press_event,
                    );
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
                    delete_commodity(
                        &commodities_register_key_press_event,
                        &globals_key_press_event,
                    );
                    Inhibit(true)
                }
                key::A => {
                    if let Some((model, iter)) = get_selection_info(
                        &commodities_register_key_press_event.core,
                        &globals_key_press_event,
                    ) {
                        let commodity_guid: String =
                            model.get_value(&iter, STORE_GUID).get().unwrap().unwrap();
                        display_linked_accounts(commodity_guid, true, &globals_key_press_event);
                        Inhibit(true)
                    } else {
                        Inhibit(false)
                    }
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
    window.set_title("Commodities");

    // Set window size
    window.get_preferred_width(); // Do these two calls to avoid annoying warnings from gtk
    window.get_preferred_height();
    window.resize(COMMODITIES_WINDOW_WIDTH, COMMODITIES_WINDOW_HEIGHT);

    window.show_all();
}
