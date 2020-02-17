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
use constants::{AccountRegister, FindCommand, FindParameters, Globals, RegisterCore};
use gdk::enums::key;
use gdk::EventType::ButtonPress;
use gdk::{EventButton, EventKey, ModifierType};
use glib::types::Type;
use gtk::prelude::{GtkListStoreExtManual, GtkMenuExtManual};
use gtk::SelectionMode::Browse;
use gtk::TreeViewGridLines::Both;
use gtk::{
    CellRendererExt, CellRendererText, CellRendererTextExt, CellRendererToggle,
    CellRendererToggleExt, ContainerExt, GtkListStoreExt, GtkMenuItemExt, GtkWindowExt, Inhibit,
    ListStore, Menu, MenuItem, MenuShellExt, ScrolledWindow, TreeModel, TreeModelExt, TreePath,
    TreeSelectionExt, TreeView, TreeViewColumn, TreeViewColumnExt, TreeViewExt, WidgetExt, Window,
    WindowType, NONE_ADJUSTMENT,
};
use queries::{
    DELETE_TRANSACTION_SPLITS_SQL, DELETE_TRANSACTION_SQL, DUPLICATE_TRANSACTION_NO_DATE_SQL,
    DUPLICATE_TRANSACTION_SPLITS_SQL, DUPLICATE_TRANSACTION_WITH_DATE_SQL,
    INCREMENT_TRANSACTION_DATE_SQL, MARKETABLE_ACCOUNT_REGISTER_SQL, NEW_TRANSACTION_SPLIT_SQL,
    NEW_TRANSACTION_SQL, NON_MARKETABLE_ACCOUNT_REGISTER_SQL, RECONCILED_BALANCE_SQL,
    TOGGLE_TRANSACTION_R_FLAG_SQL, TRANSACTION_DATE_TODAY_SQL,
    TRANSACTION_DATE_TO_END_OF_MONTH_SQL, TRANSACTION_DATE_TO_FIRST_OF_MONTH_SQL,
    TRANSACTION_DATE_TO_USER_ENTRY_SQL,
};
use rusqlite::{params, Statement};
use rust_library::constants::SPLIT_FLAG_RECONCILED;
use rust_library::guid_to_path;
use rust_library::queries::{GUID_TO_PATH_SQL, NEW_UUID_SQL};
use std::cell::RefCell;
use std::rc::Rc;
use stock_splits::get_split_factor;
use transaction::create_transaction_register;
use utilities::{
    column_index_to_column, create_tree_view_text_column, create_tree_view_toggle_column,
    date_edited, display_message_dialog, find, get_boolean_column_via_path, get_selection_info,
    get_string_column_via_path, select_last_row, select_row_by_guid,
    update_boolean_column_via_path, update_string_column_via_path,
};

// Constants

// Columns in the Account register view
const VIEW_DATE: i32 = 0;
const VIEW_NUM: i32 = VIEW_DATE + 1;
const VIEW_DESCRIPTION: i32 = VIEW_NUM + 1;
const VIEW_R: i32 = VIEW_DESCRIPTION + 1;
const VIEW_VALUE: i32 = VIEW_R + 1;
const VIEW_BALANCE: i32 = VIEW_VALUE + 1;
// Columns in the account register view for accounts that are marketable
const SHARES_VIEW_QUANTITY: i32 = VIEW_VALUE;
const SHARES_VIEW_PRICE: i32 = SHARES_VIEW_QUANTITY + 1;
const SHARES_VIEW_VALUE: i32 = SHARES_VIEW_PRICE + 1;
const SHARES_VIEW_BALANCE: i32 = SHARES_VIEW_VALUE + 1;

// Account register query columns
const QUERY_DATE: usize = 0;
const QUERY_NUM: usize = QUERY_DATE + 1;
const QUERY_DESCRIPTION: usize = QUERY_NUM + 1;
const QUERY_FLAGS: usize = QUERY_DESCRIPTION + 1;
const QUERY_TRANSACTION_GUID: usize = QUERY_FLAGS + 1;
const QUERY_VALUE: usize = QUERY_TRANSACTION_GUID + 1;
// Columns returned by the account register query for accounts that are marketable
const SHARES_QUERY_QUANTITY: usize = QUERY_VALUE + 1;
const SHARES_QUERY_SPLIT_GUID: usize = SHARES_QUERY_QUANTITY + 1;

// Columns in the account store
const STORE_DATE: i32 = 0;
const STORE_NUM: i32 = STORE_DATE + 1;
const STORE_DESCRIPTION: i32 = STORE_NUM + 1;
const STORE_R: i32 = STORE_DESCRIPTION + 1;
const STORE_TRANSACTION_GUID: i32 = STORE_R + 1;
const STORE_VALUE: i32 = STORE_TRANSACTION_GUID + 1;
// This is the value balance for registers of non-marketable accounts, quantity balance for marketable accounts
const STORE_BALANCE: i32 = STORE_VALUE + 1;
// These are additional columns in stores for marketable accounts
const STORE_QUANTITY: i32 = STORE_BALANCE + 1;
const STORE_PRICE: i32 = STORE_QUANTITY + 1;

// Store names and type for finds
// NB These must be kept in sync with the actual column definitions
const NON_MARKETABLE_STORE_COLUMN_NAMES: [&str; 5] =
    ["Date", "Num", "Description", "Reconcile State", "Value"];
const NON_MARKETABLE_STORE_COLUMN_INDICES: [i32; 5] =
    [STORE_DATE, STORE_NUM, STORE_DESCRIPTION, STORE_R, STORE_VALUE];
const NON_MARKETABLE_STORE_COLUMN_TYPES: [Type; 5] =
    [Type::String, Type::String, Type::String, Type::Bool, Type::String];
const MARKETABLE_STORE_COLUMN_NAMES: [&str; 7] =
    ["Date", "Num", "Description", "Reconcile State", "Value", "Quantity", "Price"];
const MARKETABLE_STORE_COLUMN_INDICES: [i32; 7] =
    [STORE_DATE, STORE_NUM, STORE_DESCRIPTION, STORE_R, STORE_VALUE, STORE_QUANTITY, STORE_PRICE];
const MARKETABLE_STORE_COLUMN_TYPES: [Type; 7] = [
    Type::String,
    Type::String,
    Type::String,
    Type::Bool,
    Type::String,
    Type::String,
    Type::String,
];

const ACCOUNT_WINDOW_HEIGHT: i32 = 400;
const ACCOUNT_WINDOW_WIDTH: i32 = 1000;

fn display_reconciled_balance(account_register: &AccountRegister, globals: &Globals) {
    let reconciled_balance: f64 = prepare_statement!(RECONCILED_BALANCE_SQL, globals)
        .query_row(params![account_register.guid], get_result!(f64))
        .unwrap();
    let reconciled_balance_string = format!(
        "Reconciled balance for\n{}\n\n${:.*}",
        guid_to_path(prepare_statement!(GUID_TO_PATH_SQL, globals), &account_register.guid),
        2,
        reconciled_balance
    );
    display_message_dialog(&reconciled_balance_string, globals);
}

fn delete_transaction(account_register: &AccountRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&account_register.core, globals) {
        let transaction_guid: String =
            model.get_value(&iter, STORE_TRANSACTION_GUID).get().unwrap().unwrap();
        // Before deleting the transaction, check whether there are any open transaction registers
        // pointing at this transaction. If so, they must be closed
        {
            let mut n_transaction_registers = 0;
            let transaction_registers = globals.transaction_registers.borrow_mut();
            for (guid, _) in transaction_registers.iter() {
                if guid == &transaction_guid {
                    n_transaction_registers += 1;
                }
            }
            if n_transaction_registers > 0 {
                display_message_dialog(
                    "You are attempting to delete a transaction for which \
                                        there are
open transaction registers. Please close those registers and try again.",
                    globals,
                );
                return;
            }
        }

        // Try to find an adjacent transaction to select after the deletion. The problem is that just doing
        // iter_next or iter_previous is not sufficient, because if we are deleting the first transaction, iter_previous
        // will wrap to the last. Similarly for iter_next. So need to detect those cases.
        let mut adjacent_transaction_guid = None;
        let last_row_index: usize = model.iter_n_children(None) as usize - 1;
        // Do nothing if we are deleting the last transaction. Otherwise, find the adjacent transaction.
        if last_row_index > 0 {
            let path_string = model.get_string_from_iter(&iter).unwrap();
            let adjacent_iter = iter.clone();
            let row_index: usize = path_string.parse().unwrap();
            // If deleting last row, try to select previous. Otherwise, select next.
            if row_index == last_row_index {
                if !model.iter_previous(&adjacent_iter) {
                    panic!(
                        "delete_transaction: tried to delete last transaction with a \
                         transaction count > 1, but model.iter_previous failed"
                    );
                }
            } else if !model.iter_next(&adjacent_iter) {
                panic!(
                    "delete_transaction: tried to delete transaction with a transaction count \
                     > 1, but model.iter_next failed"
                );
            }
            adjacent_transaction_guid =
                Some(model.get_value(&adjacent_iter, STORE_TRANSACTION_GUID).get().unwrap());
        }

        // Delete all splits pointing at this transaction
        {
            prepare_statement!(DELETE_TRANSACTION_SPLITS_SQL, globals)
                .execute(params![transaction_guid])
                .unwrap();
        }

        // Now delete the transaction itself
        {
            prepare_statement!(DELETE_TRANSACTION_SQL, globals)
                .execute(params![transaction_guid])
                .unwrap();
        }
        if let Some(guid) = adjacent_transaction_guid {
            refresh_account_registers(None, Some(&guid.unwrap()), globals);
        } else {
            refresh_account_registers(None, None, globals);
        };
    }
}

fn duplicate_transaction(account_register: &AccountRegister, today_p: bool, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&account_register.core, globals) {
        let new_transaction_guid = prepare_statement!(NEW_UUID_SQL, globals)
            .query_row(params![], get_result!(string))
            .unwrap();
        let source_transaction_guid: String =
            model.get_value(&iter, STORE_TRANSACTION_GUID).get().unwrap().unwrap();
        // Create the new transaction with new guid
        if today_p {
            prepare_statement!(DUPLICATE_TRANSACTION_NO_DATE_SQL, globals)
                .execute(params![new_transaction_guid, source_transaction_guid])
                .unwrap();
        } else {
            let source_transaction_date: String =
                model.get_value(&iter, STORE_DATE).get().unwrap().unwrap();
            if let Some(new_date) =
                display_calendar(&source_transaction_date, &account_register.core.window, globals)
            {
                prepare_statement!(DUPLICATE_TRANSACTION_WITH_DATE_SQL, globals)
                    .execute(params![new_transaction_guid, source_transaction_guid, &new_date])
                    .unwrap();
            } else {
                return;
            }
        }
        // Now copy the splits
        prepare_statement!(DUPLICATE_TRANSACTION_SPLITS_SQL, globals)
            .execute(params![new_transaction_guid, source_transaction_guid])
            .unwrap();
        refresh_account_registers(None, Some(&new_transaction_guid), globals);
    }
}

// Called when calendar requested for transaction
fn display_calendar_for_transaction(account_register: &AccountRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&account_register.core, globals) {
        let current_date: String = model.get_value(&iter, STORE_DATE).get().unwrap().unwrap();
        if let Some(new_date) =
            display_calendar(&current_date, &account_register.core.window, globals)
        {
            let transaction_guid: String =
                model.get_value(&iter, STORE_TRANSACTION_GUID).get().unwrap().unwrap();
            date_edited(
                &transaction_guid,
                prepare_statement!(INCREMENT_TRANSACTION_DATE_SQL, globals),
                prepare_statement!(TRANSACTION_DATE_TO_FIRST_OF_MONTH_SQL, globals),
                prepare_statement!(TRANSACTION_DATE_TO_END_OF_MONTH_SQL, globals),
                prepare_statement!(TRANSACTION_DATE_TODAY_SQL, globals),
                prepare_statement!(TRANSACTION_DATE_TO_USER_ENTRY_SQL, globals),
                &new_date,
                &globals,
            );
            refresh_account_registers(None, Some(&transaction_guid), &globals);
        }
    }
}

// Called when num field is edited
fn num_field_edited(
    path: &TreePath, new_field: &str, account_register: &AccountRegister, globals: &Globals,
) {
    let transaction_guid: String = get_string_column_via_path(
        &account_register.core.view.get_model().unwrap(),
        path,
        STORE_TRANSACTION_GUID,
    );

    // Update the database
    prepare_statement!("update transactions set num = ?1 where guid = ?2", globals)
        .execute(params![new_field.to_string(), transaction_guid])
        .unwrap();

    // Write new value to store
    update_string_column_via_path(&account_register.store, path, new_field, STORE_NUM);

    refresh_account_registers(Some(account_register), Some(&transaction_guid), &globals);
}

// Called when description field is edited
fn description_field_edited(
    path: &TreePath, new_field: &str, account_register: &AccountRegister, globals: &Globals,
) {
    let transaction_guid: String = get_string_column_via_path(
        &account_register.core.view.get_model().unwrap(),
        path,
        STORE_TRANSACTION_GUID,
    );

    // Update the database
    prepare_statement!("update transactions set description = ?1 where guid = ?2", globals)
        .execute(params![new_field.to_string(), transaction_guid])
        .unwrap();

    // Write new value to store
    update_string_column_via_path(&account_register.store, path, new_field, STORE_DESCRIPTION);

    refresh_account_registers(Some(account_register), Some(&transaction_guid), &globals);
}

// Called when a new transaction is requested
fn new_transaction(account_register: &AccountRegister, globals: &Globals) {
    let transaction_guid = prepare_statement!(NEW_UUID_SQL, globals)
        .query_row(params![], get_result!(string))
        .unwrap();
    // We will insert the new transaction into the database, together with two associated splits, one for the account
    // displayed in the register, the other unspecified.
    prepare_statement!(NEW_TRANSACTION_SQL, globals)
        .execute(params![transaction_guid, account_register.guid])
        .unwrap();
    // And the splits
    let stmt: &mut Statement = prepare_statement!(NEW_TRANSACTION_SPLIT_SQL, globals);
    stmt.execute(params![transaction_guid, account_register.guid]).unwrap();
    stmt.execute(params![transaction_guid, globals.unspecified_account_guid]).unwrap();
    refresh_account_registers(None, Some(&transaction_guid), globals);
}

// Called when 'toggled' is signalled for R column
fn r_toggled(path: &TreePath, account_register: &AccountRegister, globals: &Globals) {
    let transaction_guid: String =
        get_string_column_via_path(&account_register.store, path, STORE_TRANSACTION_GUID);
    let current_r: bool = get_boolean_column_via_path(&account_register.store, path, STORE_R);

    // Update the database
    prepare_statement!(TOGGLE_TRANSACTION_R_FLAG_SQL, globals)
        .execute(params![transaction_guid, account_register.guid])
        .unwrap();

    // Update the model and view
    update_boolean_column_via_path(&account_register.store, path, !current_r, STORE_R);

    // And refresh the transaction registers -- there might be one open for this transaction
    refresh_account_registers(Some(account_register), Some(&transaction_guid), globals);
}

// This routine needs the ability to optionally refresh all account registers,
// possibly excepting one that is optionally specified. This is
// because many routine editing operations on transactions, such as editing the description field, don't change
// the sorting order of the register in which the editing occurred, nor do they change balances. Those simple
// operations can be handled by updating the database and the store of the register in question. But other displayed
// registers might show the edited transaction and those need to be refreshed. Refreshing can be an expensive operation
// for an account register with a lot of transactions. But the situation just described is unlikely. Most of the time,
// only the register where the editing is done is displayed, so handling this situation in a brute force way, to keep the
// code simple, makes sense. The account_register argument is used to indicate whether to refresh all open registers, or
// all but the edited one. If all registers are desired, then the account_registers argument should be NULL. If an actual
// account_register pointer is supplied, then that register will be skipped.
// This routine also needs to be able to be told how to handle post-refresh selection. That is accomplished via the first argument.
pub fn refresh_account_registers(
    maybe_skip: Option<&AccountRegister>, maybe_transaction_guid: Option<&String>,
    globals: &Globals,
) {
    fn refresh_account_register(
        account_register: &AccountRegister, maybe_transaction_guid: Option<&String>,
        globals: &Globals,
    ) {
        // Record the current cursor information
        let (_, maybe_column) = account_register.core.view.get_cursor();
        // Detach the store/model from the view
        let temp: Option<&TreeModel> = None;
        account_register.core.view.set_model(temp);
        // Clear the store
        account_register.store.clear();
        populate_account_register_store(account_register, globals);
        // Re-connect store to the view
        account_register.core.view.set_model(Some(&account_register.store));
        if let Some(transaction_guid) = maybe_transaction_guid {
            if let Some(focus_column) = maybe_column {
                select_row_by_guid(
                    &account_register.core.view,
                    &transaction_guid,
                    STORE_TRANSACTION_GUID,
                    &focus_column,
                );
            } else {
                select_row_by_guid(
                    &account_register.core.view,
                    &transaction_guid,
                    STORE_TRANSACTION_GUID,
                    &column_index_to_column(&account_register.core.view, VIEW_DATE),
                );
            }
        }
    }
    for account_register in globals.account_registers.borrow().values() {
        if let Some(account_register_to_skip) = maybe_skip {
            if account_register.guid != account_register_to_skip.guid {
                refresh_account_register(account_register, maybe_transaction_guid, globals);
            };
        } else {
            refresh_account_register(account_register, maybe_transaction_guid, globals);
        }
    }
}

fn populate_account_register_store(account_register: &AccountRegister, globals: &Globals) {
    let store = &account_register.store;
    // Set up the query that fetches transaction data to produce the account_register.
    // Marketable account?
    if account_register.shares_p {
        let mut quantity_balance: f64 = 0.0;

        // NB In the query used here the where clause allowing quantity and value to be 0 is intended to
        // allow new transactions to be displayed. I am specifically disallowing splits where quantity is zero and
        // value is not zero. Those occur in sale transactions; such splits are part of account for capital gains. If
        // they were included by this query, it would result in incorrect values (we want to include only splits
        // with non-zero quantities in the value sum).
        let stmt = prepare_statement!(MARKETABLE_ACCOUNT_REGISTER_SQL, globals);
        let marketable_iter = stmt
            .query_map(
                params![account_register.guid],
                |row| -> Result<
                    (String, String, String, i32, String, f64, f64, String),
                    rusqlite::Error,
                > {
                    Ok((
                        row.get(QUERY_DATE).unwrap(),
                        row.get(QUERY_NUM).unwrap(),
                        row.get(QUERY_DESCRIPTION).unwrap(),
                        row.get(QUERY_FLAGS).unwrap(),
                        row.get(QUERY_TRANSACTION_GUID).unwrap(),
                        row.get(QUERY_VALUE).unwrap(),
                        row.get(SHARES_QUERY_QUANTITY).unwrap(),
                        row.get(SHARES_QUERY_SPLIT_GUID).unwrap(),
                    ))
                },
            )
            .unwrap();
        for wrapped_result in marketable_iter {
            let (
                date,
                num,
                description,
                split_flags,
                transaction_guid,
                value,
                quantity,
                split_guid,
            ) = wrapped_result.unwrap();
            // Append an empty row to the list store. Iter will point to the new row
            let iter = store.append();
            // The idea with the handling of the quantity is to do the balance accumulation with integer arithmetic and then
            // divide by the denominator only when it has to be stored in the model as a double. Deferring the division
            // until the last possible moment avoids the propagation of roundoff error.
            let split_adjusted_quantity: f64 = quantity * get_split_factor(&split_guid, globals);
            quantity_balance += split_adjusted_quantity;
            let quantity_string = format!("{:.*}", 4, split_adjusted_quantity);
            let price: f64 = value / split_adjusted_quantity;
            let price_string = format!("{:.*}", 4, price);
            let value_string = format!("{:.*}", 2, value);
            let quantity_balance_string = format!("{:.*}", 4, quantity_balance);
            let reconciled_p: bool = (split_flags & SPLIT_FLAG_RECONCILED) != 0;
            // add data
            store.set(
                &iter,
                &[
                    STORE_DATE as u32,
                    STORE_NUM as u32,
                    STORE_DESCRIPTION as u32,
                    STORE_R as u32,
                    STORE_TRANSACTION_GUID as u32,
                    STORE_VALUE as u32,
                    STORE_BALANCE as u32,
                    STORE_QUANTITY as u32,
                    STORE_PRICE as u32,
                ],
                &[
                    &date,
                    &num,
                    &description,
                    &reconciled_p,
                    &transaction_guid,
                    &value_string,
                    &quantity_balance_string,
                    &quantity_string,
                    &price_string,
                ],
            );
        }
    } else {
        let mut value_balance: f64 = 0.;
        let stmt = prepare_statement!(NON_MARKETABLE_ACCOUNT_REGISTER_SQL, globals);
        let non_marketable_iter = stmt
            .query_map(
                params![account_register.guid],
                |row| -> Result<(String, String, String, i32, String, f64), rusqlite::Error> {
                    Ok((
                        row.get(QUERY_DATE).unwrap(),
                        row.get(QUERY_NUM).unwrap(),
                        row.get(QUERY_DESCRIPTION).unwrap(),
                        row.get(QUERY_FLAGS).unwrap(),
                        row.get(QUERY_TRANSACTION_GUID).unwrap(),
                        row.get(QUERY_VALUE).unwrap(),
                    ))
                },
            )
            .unwrap();
        for wrapped_result in non_marketable_iter {
            let (date, num, description, split_flags, transaction_guid, value) =
                wrapped_result.unwrap();
            // Append an empty row to the list store. Iter will point to the new row
            let iter = store.append();
            value_balance += value;
            let value_string = format!("{:.*}", 2, value);
            let value_balance_string = format!("{:.*}", 2, value_balance);
            let reconciled_p: bool = (split_flags & SPLIT_FLAG_RECONCILED) != 0;

            // add data
            store.set(
                &iter,
                &[
                    STORE_DATE as u32,
                    STORE_NUM as u32,
                    STORE_DESCRIPTION as u32,
                    STORE_R as u32,
                    STORE_TRANSACTION_GUID as u32,
                    STORE_VALUE as u32,
                    STORE_BALANCE as u32,
                ],
                &[
                    &date,
                    &num,
                    &description,
                    &reconciled_p,
                    &transaction_guid,
                    &value_string,
                    &value_balance_string,
                ],
            );
        }
    }
}

fn create_account_store(shares_p: bool) -> ListStore {
    if shares_p {
        ListStore::new(&[
            Type::String, // date
            Type::String, // num
            Type::String, // description
            Type::Bool,   // R
            Type::String, // transaction guid
            Type::String, // value
            Type::String, // balance
            Type::String, // quantity for marketable
            Type::String, /* price for marketable */
        ])
    } else {
        ListStore::new(&[
            Type::String, // date
            Type::String, // num
            Type::String, // description
            Type::Bool,   // R
            Type::String, // transaction guid
            Type::String, // value
            Type::String, /* balance */
        ])
    }
}

pub fn create_account_register(
    account_guid: String, shares_p: bool, full_account_name: &str, globals: &Rc<Globals>,
) {
    fn display_transaction_register(account_register: Rc<AccountRegister>, globals: Rc<Globals>) {
        if let Some((model, iter)) = get_selection_info(&account_register.core, &globals) {
            let transaction_guid: String =
                model.get_value(&iter, STORE_TRANSACTION_GUID).get().unwrap().unwrap();
            let description: String =
                model.get_value(&iter, STORE_DESCRIPTION).get().unwrap().unwrap();
            let date: String = model.get_value(&iter, STORE_DATE).get().unwrap().unwrap();
            create_transaction_register(
                transaction_guid,
                description,
                date.as_str(),
                &account_register,
                globals,
            );
        }
    };

    // Check to see if there is already a register open for this account
    if globals.account_registers.borrow().contains_key(&account_guid) {
        display_message_dialog(
            &format!("An account register already exists for {}", full_account_name),
            &globals,
        );
    } else {
        // Build the account register
        let account_register = Rc::new(AccountRegister {
            core: RegisterCore {
                view: TreeView::new(),
                window: Window::new(WindowType::Toplevel),
            },
            find_parameters: RefCell::new(FindParameters {
                column_index: None,
                path: None,
                regex: None,
                column_type: None,
                column_names: if shares_p {
                    &MARKETABLE_STORE_COLUMN_NAMES
                } else {
                    &NON_MARKETABLE_STORE_COLUMN_NAMES
                },
                column_indices: if shares_p {
                    &MARKETABLE_STORE_COLUMN_INDICES
                } else {
                    &NON_MARKETABLE_STORE_COLUMN_INDICES
                },
                column_types: if shares_p {
                    &MARKETABLE_STORE_COLUMN_TYPES
                } else {
                    &NON_MARKETABLE_STORE_COLUMN_TYPES
                },
                default_store_column: STORE_DESCRIPTION,
                default_view_column: VIEW_DESCRIPTION as u32,
            }),
            guid: account_guid.clone(),
            scrolled_window: ScrolledWindow::new(NONE_ADJUSTMENT, NONE_ADJUSTMENT),
            shares_p,
            store: create_account_store(shares_p),
        });

        // Record the the descriptor in the account_registers hashtable
        globals
            .account_registers
            .borrow_mut()
            .insert(account_guid.clone(), account_register.clone());

        // Populate the model/store
        populate_account_register_store(&account_register, globals);

        // Unwrap optional entries used repeatedly below
        let view = &account_register.core.view;
        let window = &account_register.core.window;
        let scrolled_window = &account_register.scrolled_window;
        let store = &account_register.store;

        // Hook up store to the view
        view.set_model(Some(store));

        // Column setup
        // Date
        {
            let renderer = CellRendererText::new();
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            renderer.connect_edited(move |_, path, new_date| {
                let transaction_guid = get_string_column_via_path(
                    &closure_account_register.store,
                    &path,
                    STORE_TRANSACTION_GUID,
                );
                date_edited(
                    &transaction_guid,
                    prepare_statement!(INCREMENT_TRANSACTION_DATE_SQL, closure_globals),
                    prepare_statement!(TRANSACTION_DATE_TO_FIRST_OF_MONTH_SQL, closure_globals),
                    prepare_statement!(TRANSACTION_DATE_TO_END_OF_MONTH_SQL, closure_globals),
                    prepare_statement!(TRANSACTION_DATE_TODAY_SQL, closure_globals),
                    prepare_statement!(TRANSACTION_DATE_TO_USER_ENTRY_SQL, closure_globals),
                    new_date,
                    &closure_globals,
                );
                refresh_account_registers(None, Some(&transaction_guid), &closure_globals);
            });
            renderer.set_property_editable(true);
            // Add column to the view
            let column: TreeViewColumn =
                create_tree_view_text_column(&renderer, "Date", STORE_DATE);
            view.insert_column(&column, VIEW_DATE);
        }
        // Num
        {
            let renderer = CellRendererText::new();
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            renderer.connect_edited(move |_, path, new_num| {
                num_field_edited(&path, new_num, &closure_account_register, &closure_globals)
            });
            renderer.set_property_editable(true);
            // Add column to the view
            let column: TreeViewColumn = create_tree_view_text_column(&renderer, "Num", STORE_NUM);
            view.insert_column(&column, VIEW_NUM);
            column.set_resizable(true);
        }
        // Description
        {
            let renderer = CellRendererText::new();
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            renderer.connect_edited(move |_, path, new_description| {
                description_field_edited(
                    &path,
                    &new_description,
                    &closure_account_register,
                    &closure_globals,
                )
            });
            renderer.set_property_editable(true);
            // Add column to the view
            let column: TreeViewColumn =
                create_tree_view_text_column(&renderer, "Description", STORE_DESCRIPTION);
            view.insert_column(&column, VIEW_DESCRIPTION);
            column.set_resizable(true);
            column.set_expand(true);
        }
        // R
        {
            let renderer = CellRendererToggle::new();
            let globals = globals.clone();
            let closure_account_register = account_register.clone();
            renderer.connect_toggled(move |_, path| {
                r_toggled(&path, &closure_account_register, &globals);
                Inhibit(false);
            });
            renderer.set_activatable(true);
            // Add column to the view
            let column: TreeViewColumn = create_tree_view_toggle_column(&renderer, "R", STORE_R);
            view.insert_column(&column, VIEW_R);
        }
        // Quantity  and price only if stock or mutual fund
        if account_register.shares_p {
            {
                let renderer = CellRendererText::new();
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Shares", STORE_QUANTITY);
                view.insert_column(&column, SHARES_VIEW_QUANTITY);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            {
                let renderer = CellRendererText::new();
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Price", STORE_PRICE);
                view.insert_column(&column, SHARES_VIEW_PRICE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            {
                let renderer = CellRendererText::new();
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Value", STORE_VALUE);
                view.insert_column(&column, SHARES_VIEW_VALUE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            {
                let renderer = CellRendererText::new();
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Share Balance", STORE_BALANCE);
                view.insert_column(&column, SHARES_VIEW_BALANCE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
        } else {
            {
                let renderer = CellRendererText::new();
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Value", STORE_VALUE);
                view.insert_column(&column, VIEW_VALUE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            {
                let renderer = CellRendererText::new();
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Balance", STORE_BALANCE);
                view.insert_column(&column, VIEW_BALANCE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
        }

        // Remove from account registers hash table when destroyed
        let delete_closure_globals = globals.clone();
        let delete_closure_account_guid = account_guid.clone();
        window.connect_delete_event(move |_, _| {
            if delete_closure_globals
                .account_registers
                .borrow_mut()
                .remove(&delete_closure_account_guid)
                .is_some()
            {
                Inhibit(false)
            } else {
                panic!(
                    "Account register deleted, but not found in account_registers hash \
                     table"
                );
            }
        });

        // Set up to handle mouse button press events
        // Build the top-level popup menu
        let account_register_menu = Menu::new();
        {
            let account_register_menu_item = MenuItem::new_with_label("New transaction (ctrl-n)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    new_transaction(&closure_account_register, &closure_globals);
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Duplicate selected transaction (calendar) (Ctrl-d)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    duplicate_transaction(&closure_account_register, false, &closure_globals);
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Duplicate selected transaction (today) (Alt-d)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    duplicate_transaction(&closure_account_register, true, &closure_globals);
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Delete selected transaction (Ctrl-Shift-d)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    delete_transaction(&closure_account_register, &closure_globals);
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Find transaction backward (Ctrl-f)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    find(
                        &FindCommand::FindBackward,
                        &closure_account_register.find_parameters,
                        &closure_account_register.core,
                        &closure_globals,
                    );
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Find next transaction backward (Ctrl-g)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    find(
                        &FindCommand::FindNextBackward,
                        &closure_account_register.find_parameters,
                        &closure_account_register.core,
                        &closure_globals,
                    );
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Find transaction forward (Ctrl-Shift-f)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    find(
                        &FindCommand::FindForward,
                        &closure_account_register.find_parameters,
                        &closure_account_register.core,
                        &closure_globals,
                    );
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Find next transaction forward (Ctrl-Shift-g)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    find(
                        &FindCommand::FindNextForward,
                        &closure_account_register.find_parameters,
                        &closure_account_register.core,
                        &closure_globals,
                    );
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Display transaction register (Ctrl-t)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    display_transaction_register(
                        closure_account_register.clone(),
                        closure_globals.clone(),
                    );
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Display calendar for selected transaction (Ctrl-a)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    display_calendar_for_transaction(&closure_account_register, &closure_globals);
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }
        {
            let account_register_menu_item =
                MenuItem::new_with_label("Display reconciled balance (Ctrl-r)");
            let closure_globals = globals.clone();
            let closure_account_register = account_register.clone();
            account_register_menu_item.connect_activate(
                move |_account_register_menu_item: &MenuItem| {
                    display_reconciled_balance(&closure_account_register, &closure_globals);
                },
            );
            account_register_menu.append(&account_register_menu_item);
        }

        view.connect_button_press_event(move |_view: &TreeView, event_button: &EventButton| {
            // single click and right button pressed?
            if (event_button.get_event_type() == ButtonPress) && (event_button.get_button() == 3) {
                account_register_menu.show_all();
                account_register_menu.popup_easy(3, event_button.get_time());
                Inhibit(true) // we handled this
            } else {
                Inhibit(false) // we did not handle this
            }
        });

        // Handle deletion of the account register
        let globals_delete_event = globals.clone();
        let account_guid_delete_event = account_guid.clone();
        window.connect_delete_event(move |_, _| {
            globals_delete_event.account_registers.borrow_mut().remove(&account_guid_delete_event);
            Inhibit(false)
        });

        // Connect to signal for key press events
        let globals_key_press_event = globals.clone();
        let account_register_key_press_event = account_register.clone();
        view.connect_key_press_event(move |_accounts_view: &TreeView, event_key: &EventKey| {
            let masked_state: u32 =
                event_key.get_state().bits() & globals_key_press_event.modifiers.bits();
            // Ctrl key pressed?
            if masked_state == ModifierType::CONTROL_MASK.bits() {
                match event_key.get_keyval() {
                    key::n => {
                        new_transaction(
                            &account_register_key_press_event,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    key::d => {
                        duplicate_transaction(
                            &account_register_key_press_event,
                            false,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    key::a => {
                        display_calendar_for_transaction(
                            &account_register_key_press_event,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    key::f => {
                        find(
                            &FindCommand::FindBackward,
                            &account_register_key_press_event.find_parameters,
                            &account_register_key_press_event.core,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    key::g => {
                        find(
                            &FindCommand::FindNextBackward,
                            &account_register_key_press_event.find_parameters,
                            &account_register_key_press_event.core,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    key::t => {
                        display_transaction_register(
                            account_register_key_press_event.clone(),
                            globals_key_press_event.clone(),
                        );
                        Inhibit(true)
                    }
                    key::r => {
                        display_reconciled_balance(
                            &account_register_key_press_event,
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
                        delete_transaction(
                            &account_register_key_press_event,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    key::F => {
                        find(
                            &FindCommand::FindForward,
                            &account_register_key_press_event.find_parameters,
                            &account_register_key_press_event.core,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    key::G => {
                        find(
                            &FindCommand::FindNextForward,
                            &account_register_key_press_event.find_parameters,
                            &account_register_key_press_event.core,
                            &globals_key_press_event,
                        );
                        Inhibit(true)
                    }
                    // Indicate we didn't handle the event
                    _ => Inhibit(false),
                }
            } else if masked_state == ModifierType::MOD1_MASK.bits() {
                match event_key.get_keyval() {
                    key::d => {
                        duplicate_transaction(
                            &account_register_key_press_event,
                            true,
                            &globals_key_press_event,
                        );
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

        // Grid lines for readability
        view.set_grid_lines(Both);

        scrolled_window.add(view);
        window.add(scrolled_window);

        // Set geometry hints
        window.get_preferred_width(); // Do these two calls to avoid annoying warnings from gtk
        window.get_preferred_height();
        window.set_default_size(ACCOUNT_WINDOW_WIDTH, ACCOUNT_WINDOW_HEIGHT);

        // Set window title to account name
        window.set_title(full_account_name);

        // Set the view's selection mode
        view.get_selection().set_mode(Browse);

        select_last_row(&view, &column_index_to_column(&view, VIEW_DATE));

        window.show_all();
    }
}
