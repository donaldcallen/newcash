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

extern crate gdk;
extern crate glib;
extern crate glib_sys;
extern crate gobject_sys;
extern crate gtk;
extern crate regex;
extern crate rusqlite;
#[macro_use]
extern crate rust_library;

mod account;
mod book;
mod calendar;
mod commodities;
mod commodity;
mod constants;
mod queries;
mod stock_splits;
mod transaction;
mod utilities;

use account::create_account_register;
use book::{
    add_account_tree_child_nodes, copy_account_guid_to_account_copy_buffer,
    copy_account_value_to_clipboard, create_accounts_model, delete_account, edit_account,
    new_account, paste_account, reparent_account, toggle_show_hidden,
};
use commodities::create_commodities_register;
use constants::{
    Globals, ACCOUNT_TREE_STORE_FLAGS, ACCOUNT_TREE_STORE_GUID, ACCOUNT_TREE_STORE_NAME,
};
use gdk::enums::key;
use gdk::EventType::ButtonPress;
use gdk::{EventButton, EventKey, ModifierType};
use glib::types::Type;
use gtk::prelude::GtkMenuExtManual;
use gtk::{
    accelerator_get_default_mod_mask, CellLayoutExt, CellRendererText, ContainerExt,
    GtkMenuItemExt, GtkWindowExt, Inhibit, Menu, MenuItem, MenuShellExt, PolicyType,
    ScrolledWindow, ScrolledWindowExt, TreeIter, TreeModelExt, TreePath, TreeStore, TreeView,
    TreeViewColumnExt, TreeViewExt, WidgetExt, Window, WindowType, NONE_ADJUSTMENT,
};
use queries::{BASIC_INFO_SQL, UNBALANCED_TRANSACTIONS_SQL};
use rusqlite::{params, Connection, LoadExtensionGuard};
use rust_library::constants::{
    ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS, ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE,
    ACCOUNT_FLAG_PLACEHOLDER, EPSILON,
};
use rust_library::queries::{GUID_TO_PATH_SQL, INHERITED_P_SQL};
use rust_library::{guid_to_path, inherited_p};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;
use std::rc::Rc;
use utilities::display_message_dialog;

// Constants

const ACCOUNTS_WINDOW_MIN_HEIGHT: i32 = 600;
const ACCOUNTS_WINDOW_MIN_WIDTH: i32 = 340;
const EXTENSIONS_LIBRARY_FILE_INDEX: usize = 1;
const DB_PATH_INDEX: usize = EXTENSIONS_LIBRARY_FILE_INDEX + 1;
const N_ARGS: usize = DB_PATH_INDEX + 1;

fn main() {
    let actual_arg_count = env::args().count();
    // Check that the number of arguments is correct
    if actual_arg_count != N_ARGS {
        panic!(
            "Incorrect number of command line arguments: {}. Should  be {}.
Usage: newcash pathToExtensionsLibrary pathToDatabase",
            actual_arg_count, N_ARGS
        );
    }

    let db_path = env::args().nth(DB_PATH_INDEX).unwrap();
    let db = Connection::open(&db_path).unwrap();
    let (root_account_guid, book_name, unspecified_account_guid) =
        db.query_row(BASIC_INFO_SQL, params![], get_result!(string_string_string)).unwrap();

    // Initialize gtk
    gtk::init().unwrap();

    // Here is the problem: the fields in globals are used throughout this program. Most of the program is executed
    // as a result of callbacks being invoked as a response to signals and events. The initial code executed
    // by the callbacks is in closures that are passed to the signal connect methods. Those closures have static lifetimes,
    // specified by the definitions of the connect methods in the gtk crate. Since those callbacks use these
    // the values must all live as long as the closures. One way to accomplish this is to define them as static
    // global variables. The problem with that is that mutable statics are unsafe.
    // So instead, I define the globals here as Rcs, to be cloned before each of the connect
    // methods is invoked and the closures passed to them are move closures. So each closure owns its own copy of the pointers
    // to these variables. Rc is inherently read-only, because it hands out multiple pointers. For mutable variables,
    // RefCell is used, from which mutable borrows can be obtained.
    //
    // Also note that the accounts_store needs to be recorded here. The model is available from the view,
    // but while a store is a model, a model is not a store. Thus the store cannot be obtained from the view.
    // There are operations that you can perform on a store but not a model and
    // those operations are needed in Newcash. This makes it necessary to record the store in the globals structure.
    let globals = Rc::new(Globals {
        account_copy_buffer: RefCell::new(None),
        account_registers: RefCell::new(HashMap::new()),
        accounts_store: TreeStore::new(&[Type::String, Type::String, Type::I32]),
        accounts_view: TreeView::new(),
        accounts_window: Window::new(WindowType::Toplevel),
        book_name,
        db,
        db_path,
        guid_processed: RefCell::new(HashSet::new()),
        guid_to_full_path: RefCell::new(HashMap::new()),
        modifiers: accelerator_get_default_mod_mask(),
        root_account_guid: Rc::new(root_account_guid),
        show_hidden: RefCell::new(false),
        transaction_registers: RefCell::new(HashMap::new()),
        unspecified_account_guid,
    });

    // Load sqlite extensions, so we have math functions
    let unix_extensions_file_path = env::args().nth(EXTENSIONS_LIBRARY_FILE_INDEX).unwrap();
    let extensions_file_path = Path::new(&unix_extensions_file_path);
    {
        let _guard = LoadExtensionGuard::new(&globals.db).unwrap();
        &globals.db.load_extension(extensions_file_path, None).unwrap();
    }

    // Set rusqlite cache capacity
    &globals.db.set_prepared_statement_cache_capacity(200);

    // Create the accounts window
    let accounts_renderer = CellRendererText::new();
    let scrolled_window = ScrolledWindow::new(NONE_ADJUSTMENT, NONE_ADJUSTMENT);

    // Add the Accounts column to the view
    let accounts_column = gtk::TreeViewColumn::new();
    accounts_column.pack_start(&accounts_renderer, true);
    accounts_column.add_attribute(&accounts_renderer, "text", ACCOUNT_TREE_STORE_NAME);
    accounts_column.set_title("Accounts");
    globals.accounts_view.insert_column(&accounts_column, 0);

    // Make the header invisible
    globals.accounts_view.set_headers_visible(false);

    create_accounts_model(&globals);

    // Hook up the store to the view
    globals.accounts_view.set_model(Some(&globals.accounts_store));

    scrolled_window.add(&globals.accounts_view);
    scrolled_window.set_policy(PolicyType::Never, PolicyType::Automatic);
    globals.accounts_window.add(&scrolled_window);

    // Set default size
    globals.accounts_window.set_default_size(ACCOUNTS_WINDOW_MIN_WIDTH, ACCOUNTS_WINDOW_MIN_HEIGHT);

    // Set window title
    globals
        .accounts_window
        .set_title(format!("{} ({})", &globals.db_path, &globals.book_name).as_str());

    // Enable interactive search
    globals.accounts_view.set_enable_search(true);
    globals.accounts_view.set_search_column(ACCOUNT_TREE_STORE_NAME);

    // Make sure expanders are visible
    globals.accounts_view.set_show_expanders(true);

    // Expand root node */
    let path = TreePath::new_from_string("0");
    globals.accounts_view.expand_row(&path, false);

    globals.accounts_window.show_all();

    // Set up callback to handle deletion of the accounts window
    let globals_delete_event = globals.clone();
    globals.accounts_window.connect_delete_event(move |_, _| {
        let mut non_zero_balance = false;
        for transaction_register in globals_delete_event.transaction_registers.borrow().values() {
            if prepare_statement!(UNBALANCED_TRANSACTIONS_SQL, globals_delete_event)
                .query_row(params![&*transaction_register.guid], get_result!(f64))
                .unwrap()
                > EPSILON
            {
                non_zero_balance = true;
                break;
            }
        }
        if non_zero_balance {
            utilities::display_message_dialog(
                "You have at least one open \
                 transaction register with \
                 a non-zero \
                 balance.\nPlease properly \
                 balance the transaction(s) \
                 before attempting to quit \
                 from Newcash",
                &globals_delete_event,
            );
            Inhibit(true)
        } else {
            gtk::main_quit();
            // Let the default handler destroy the window.
            Inhibit(false)
        }
    });

    // Set up to handle test_expand_row signals on the accounts window's tree view
    let globals_test_expand_row = globals.clone();
    globals.accounts_view.connect_test_expand_row(
        move |_view: &TreeView, iter: &TreeIter, _path: &TreePath| {
            if let Some(child_iter) =
                globals_test_expand_row.accounts_store.iter_children(Some(iter))
            {
                loop {
                    // Obtain the guid of the child account
                    let child_guid: String = globals_test_expand_row
                        .accounts_store
                        .get_value(&child_iter, ACCOUNT_TREE_STORE_GUID)
                        .get()
                        .unwrap()
                        .unwrap();
                    // And add its child to the store, the grandchild of the clicked node
                    add_account_tree_child_nodes(
                        &child_guid,
                        &child_iter,
                        &globals_test_expand_row,
                    );
                    if !globals_test_expand_row.accounts_store.iter_next(&child_iter) {
                        break;
                    }
                }
            }
            Inhibit(false)
        },
    );

    // Set up to handle row_activated signals on the accounts window's tree view
    let globals_row_activated = globals.clone();
    globals.accounts_view.connect_row_activated(move |_, path: &TreePath, _| {
        if let Some(iter) = globals_row_activated.accounts_store.get_iter(&path) {
            let flags: i32 = globals_row_activated
                .accounts_store
                .get_value(&iter, ACCOUNT_TREE_STORE_FLAGS)
                .get()
                .unwrap()
                .unwrap();
            if (flags & ACCOUNT_FLAG_PLACEHOLDER) != 0 {
                display_message_dialog(
                    "Be aware that you have requested an \
                     account register for a placeholder \
                     account.",
                    &globals_row_activated,
                );
            }
            let account_guid: String = globals_row_activated
                .accounts_store
                .get_value(&iter, ACCOUNT_TREE_STORE_GUID)
                .get()
                .unwrap()
                .unwrap();
            let marketable_p: bool;
            let path: String;
            {
                let inherited_p_stmt = prepare_statement!(INHERITED_P_SQL, globals_row_activated);
                marketable_p = inherited_p(
                    inherited_p_stmt,
                    &account_guid,
                    ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS,
                ) && inherited_p(
                    inherited_p_stmt,
                    &account_guid,
                    ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE,
                );
                path = guid_to_path(
                    prepare_statement!(GUID_TO_PATH_SQL, globals_row_activated),
                    &account_guid,
                );
            }
            create_account_register(
                account_guid,
                marketable_p,
                path.as_str(),
                &globals_row_activated,
            );
        }
    });

    // Set up to handle mouse button press events
    // Build the top-level popup menu
    let accounts_menu = Menu::new();

    {
        let accounts_menu_item = MenuItem::new_with_label("New account (Ctrl-n)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            new_account(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item = MenuItem::new_with_label("Edit account (Ctrl-e)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            edit_account(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item =
            MenuItem::new_with_label("Copy account name to system clipboard (Ctrl-c)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            copy_account_value_to_clipboard(
                ACCOUNT_TREE_STORE_NAME,
                "Improper selection. Cannot \
                 perform requested copy of account \
                 name.",
                false,
                &globals,
            );
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item =
            MenuItem::new_with_label("Copy account path to system clipboard (Ctrl-Shift-c)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            copy_account_value_to_clipboard(
                ACCOUNT_TREE_STORE_GUID,
                "Improper selection. Cannot \
                 perform requested copy of account \
                 path.",
                true,
                &globals,
            );
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item =
            MenuItem::new_with_label("Copy account guid to system clipboard (Ctrl-Shift-g)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            copy_account_value_to_clipboard(
                ACCOUNT_TREE_STORE_GUID,
                "Improper selection. Cannot \
                 perform requested copy of account \
                 guid.",
                false,
                &globals,
            );
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item =
            MenuItem::new_with_label("Copy account to Newcash clipboard (Alt-c)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            copy_account_guid_to_account_copy_buffer(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item =
            MenuItem::new_with_label("Paste account from Newcash clipboard (Alt-v)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            paste_account(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item = MenuItem::new_with_label("Re-parent account (Ctrl-r)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            reparent_account(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item = MenuItem::new_with_label("Delete account (Ctrl-Shift-d)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            delete_account(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item = MenuItem::new_with_label("Display commodities (Ctrl-m)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            create_commodities_register(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }
    {
        let accounts_menu_item = MenuItem::new_with_label("Toggle show hidden accounts (Ctrl-h)");
        let globals = globals.clone();
        accounts_menu_item.connect_activate(move |_accounts_menu_item: &MenuItem| {
            toggle_show_hidden(&globals);
        });
        accounts_menu.append(&accounts_menu_item);
    }

    globals.accounts_view.connect_button_press_event(
        move |_accounts_view: &TreeView, event_button: &EventButton| {
            // single click and right button pressed?
            if (event_button.get_event_type() == ButtonPress) && (event_button.get_button() == 3) {
                accounts_menu.show_all();
                accounts_menu.popup_easy(3, event_button.get_time());
                Inhibit(true) // we handled this
            } else {
                Inhibit(false) // we did not handle this
            }
        },
    );

    // Connect to signal for key press events
    let key_press_globals = globals.clone();
    globals.accounts_view.connect_key_press_event(
        move |_accounts_view: &TreeView, event_key: &EventKey| {
            let masked_state: u32 =
                event_key.get_state().bits() & key_press_globals.modifiers.bits();
            // Ctrl key pressed?
            if masked_state == ModifierType::CONTROL_MASK.bits() {
                match event_key.get_keyval() {
                    key::n => {
                        new_account(&key_press_globals);
                        Inhibit(true)
                    }
                    key::e => {
                        edit_account(&key_press_globals);
                        Inhibit(true)
                    }
                    key::r => {
                        reparent_account(&key_press_globals);
                        Inhibit(true)
                    }
                    key::c => {
                        copy_account_value_to_clipboard(
                            ACCOUNT_TREE_STORE_NAME,
                            "Improper selection. Cannot perform \
                             requested copy of account name.",
                            false,
                            &key_press_globals,
                        );
                        Inhibit(true)
                    }
                    key::h => {
                        toggle_show_hidden(&key_press_globals);
                        Inhibit(true)
                    }
                    key::m => {
                        create_commodities_register(&key_press_globals);
                        Inhibit(true)
                    }
                    // Indicate we didn't handle the event
                    _ => Inhibit(false),
                }
            } else if masked_state
                == (ModifierType::CONTROL_MASK.bits() | ModifierType::SHIFT_MASK.bits())
            {
                match event_key.get_keyval() {
                    key::C => {
                        copy_account_value_to_clipboard(
                            ACCOUNT_TREE_STORE_GUID,
                            "Improper selection. Cannot perform \
                             requested copy of account path.",
                            true,
                            &key_press_globals,
                        );
                        Inhibit(true)
                    }
                    key::G => {
                        copy_account_value_to_clipboard(
                            ACCOUNT_TREE_STORE_GUID,
                            "Improper selection. Cannot perform \
                             requested copy of account guid.",
                            false,
                            &key_press_globals,
                        );
                        Inhibit(true)
                    }
                    key::D => {
                        delete_account(&key_press_globals);
                        Inhibit(true)
                    }
                    // Indicate we didn't handle the event
                    _ => Inhibit(false),
                }
            } else if masked_state == ModifierType::MOD1_MASK.bits() {
                match event_key.get_keyval() {
                    key::c => {
                        copy_account_guid_to_account_copy_buffer(&key_press_globals);
                        Inhibit(true)
                    }
                    key::v => {
                        paste_account(&key_press_globals);
                        Inhibit(true)
                    }
                    // Indicate we didn't handle the event
                    _ => Inhibit(false),
                }
            } else {
                // We didn't handle the event
                Inhibit(false)
            }
        },
    );

    // Run the main gtk loop
    gtk::main();
}
