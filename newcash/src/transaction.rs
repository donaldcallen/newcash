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

use account::{create_account_register, refresh_account_registers};
use constants::WhatChanged::{AccountNameChanged, SplitEdited, TransactionChanged};
use constants::{AccountRegister, Globals, RegisterCore, TransactionRegister, WhatChanged};
use gdk::enums::key;
use gdk::Atom;
use gdk::EventType::ButtonPress;
use gdk::{EventButton, EventKey, ModifierType};
use gtk::SelectionMode::Browse;
use gtk::TreeViewGridLines::Both;
use gtk::{
    CellRendererExt, CellRendererText, CellRendererTextExt, CellRendererToggle,
    CellRendererToggleExt, Clipboard, ContainerExt, Dialog, DialogExt, DialogFlags,
    GtkListStoreExt, GtkListStoreExtManual, GtkMenuExtManual, GtkMenuItemExt, GtkWindowExt,
    Inhibit, Label, ListStore, Menu, MenuItem, MenuShellExt, ResponseType, TreeModel, TreeModelExt,
    TreePath, TreeSelectionExt, TreeView, TreeViewColumn, TreeViewColumnExt, TreeViewExt, Type,
    WidgetExt, Window, WindowType,
};
use queries::{
    BALANCE_TRANSACTION_SQL, CHECK_TRANSACTION_BALANCE_SQL, DELETE_SPLIT_SQL, DUPLICATE_SPLIT_SQL,
    GET_BALANCING_SPLIT_GUIDS_SQL, MARKETABLE_TRANSACTION_REGISTER_SQL, MONEY_MARKET_P_SQL,
    NEW_SPLIT_SQL, NON_MARKETABLE_TRANSACTION_REGISTER_SQL, PASTE_ACCOUNT_GUID_SQL,
    PRICE_EDITED_NULL_CHECK_SQL, REVERSE_SIGN_SQL, SPLIT_COUNT_SQL, TOGGLE_SPLIT_R_FLAG_SQL,
    TOGGLE_SPLIT_T_FLAG_SQL, UPDATE_BALANCING_MONEY_MARKET_SPLIT_SQL,
    UPDATE_BALANCING_SPLIT_VALUE_SQL, UPDATE_MEMO_SQL, UPDATE_MONEY_MARKET_VALUE_QUANTITY_SQL,
    UPDATE_QUANTITY_SQL, UPDATE_VALUE_SQL,
};
use rusqlite::{params, Statement};
use rust_library::constants::{
    ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE, COMMODITY_FLAG_MONEY_MARKET_FUND, EPSILON,
    SPLIT_FLAG_RECONCILED, SPLIT_FLAG_TRANSFER,
};
use rust_library::queries::{GUID_TO_PATH_SQL, INHERITED_P_SQL, NEW_UUID_SQL};
use rust_library::{guid_to_path, inherited_p};
use std::rc::Rc;
use stock_splits::get_split_factor;
use utilities::{
    column_index_to_column, create_tree_view_text_column, create_tree_view_toggle_column,
    display_message_dialog, evaluate_expression, get_selection_info, get_string_column_via_path,
    select_first_row, select_last_row, select_row_by_guid, update_boolean_column_via_path,
    update_string_column_via_path,
};

// Transaction register view columns
const VIEW_FULL_ACCOUNT_NAME: i32 = 0;
const VIEW_MEMO: i32 = VIEW_FULL_ACCOUNT_NAME + 1;
const VIEW_R: i32 = VIEW_MEMO + 1;
const VIEW_VALUE: i32 = VIEW_R + 1;
const VIEW_VALUE_BALANCE: i32 = VIEW_VALUE + 1;
const SHARES_VIEW_T: i32 = VIEW_R + 1;
const SHARES_VIEW_QUANTITY: i32 = SHARES_VIEW_T + 1;
const SHARES_VIEW_PRICE: i32 = SHARES_VIEW_QUANTITY + 1;
const SHARES_VIEW_VALUE: i32 = SHARES_VIEW_PRICE + 1;
const SHARES_VIEW_VALUE_BALANCE: i32 = SHARES_VIEW_VALUE + 1;

// Transaction register query columns
const QUERY_ACCOUNT_GUID: usize = 0;
const QUERY_SPLIT_GUID: usize = QUERY_ACCOUNT_GUID + 1;
const QUERY_MEMO: usize = QUERY_SPLIT_GUID + 1;
const QUERY_FLAGS: usize = QUERY_MEMO + 1;
const QUERY_VALUE: usize = QUERY_FLAGS + 1;
const QUERY_QUANTITY: usize = QUERY_VALUE + 1;

// Transaction register store columns
const STORE_ACCOUNT_GUID: i32 = 0;
const STORE_SPLIT_GUID: i32 = STORE_ACCOUNT_GUID + 1;
const STORE_TRANSACTION_GUID: i32 = STORE_SPLIT_GUID + 1;
const STORE_FULL_ACCOUNT_NAME: i32 = STORE_TRANSACTION_GUID + 1;
const STORE_MEMO: i32 = STORE_FULL_ACCOUNT_NAME + 1;
const STORE_R: i32 = STORE_MEMO + 1;
const STORE_VALUE: i32 = STORE_R + 1;
const STORE_BALANCE: i32 = STORE_VALUE + 1;
const STORE_QUANTITY: i32 = STORE_BALANCE + 1;
const STORE_PRICE: i32 = STORE_QUANTITY + 1;
const STORE_T: i32 = STORE_PRICE + 1;

fn select_after_refresh(transaction_register: &TransactionRegister) -> i32 {
    if transaction_register.account_register.shares_p {
        SHARES_VIEW_QUANTITY
    } else {
        VIEW_VALUE
    }
}

fn new_split(transaction_register: &TransactionRegister, globals: &Globals) {
    let view = &transaction_register.core.view;
    // We will insert the new split into the database with an initial value that will balance the transaction.
    // Once this is complete, we will update the transaction register
    // with the new split selected. We will also scroll to it.
    let new_split_guid = prepare_statement!(NEW_UUID_SQL, globals).query_row(params![],
                                                                             get_result!(string))
                                                                  .unwrap();
    prepare_statement!(NEW_SPLIT_SQL, globals).execute(params![new_split_guid,
                                                               transaction_register.guid,
                                                               globals.unspecified_account_guid])
                                              .unwrap();
    refresh_transaction_registers(&TransactionChanged, &transaction_register.guid, globals);
    select_row_by_guid(view,
                       &new_split_guid,
                       STORE_SPLIT_GUID,
                       &column_index_to_column(view, select_after_refresh(&transaction_register)));
}

fn duplicate_split(transaction_register: &TransactionRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&transaction_register.core, globals) {
        let view = &transaction_register.core.view;
        let source_split_guid: String = model.get_value(&iter, STORE_SPLIT_GUID).get().unwrap();
        let destination_split_guid =
            prepare_statement!(NEW_UUID_SQL, globals).query_row(params![], get_result!(string))
                                                     .unwrap();
        // Create the new split
        prepare_statement!(DUPLICATE_SPLIT_SQL, globals).execute(params![destination_split_guid,
                                                                         source_split_guid])
                                                        .unwrap();
        refresh_transaction_registers(&TransactionChanged, &transaction_register.guid, globals);
        select_row_by_guid(view,
                           &destination_split_guid,
                           STORE_SPLIT_GUID,
                           &column_index_to_column(view,
                                                   select_after_refresh(&transaction_register)));
    }
}

fn delete_split(transaction_register: &TransactionRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&transaction_register.core, globals) {
        let n_rows: usize = model.iter_n_children(None) as usize;
        let view = &transaction_register.core.view;
        let split_guid: String = model.get_value(&iter, STORE_SPLIT_GUID).get().unwrap();
        prepare_statement!(DELETE_SPLIT_SQL, globals).execute(params![split_guid])
                                                     .unwrap();
        refresh_transaction_registers(&TransactionChanged, &transaction_register.guid, globals);
        refresh_account_registers(None, Some(&transaction_register.guid), globals);
        // Don't try to select if we just deleted the last row
        if n_rows > 1 {
            select_last_row(view,
                            &column_index_to_column(view,
                                                    select_after_refresh(&transaction_register)));
        }
    }
}

// This can only be done for non-marketable accounts, or marketable accounts when value is not the dependent variable
fn balance_transaction(transaction_register: &TransactionRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&transaction_register.core, globals) {
        let view = &transaction_register.core.view;
        let split_guid: String = model.get_value(&iter, STORE_SPLIT_GUID).get().unwrap();
        prepare_statement!(BALANCE_TRANSACTION_SQL, globals)
            .execute(params![transaction_register.guid, &split_guid])
            .unwrap();
        refresh_transaction_registers(&TransactionChanged, &transaction_register.guid, globals);
        select_row_by_guid(view,
                           &split_guid,
                           STORE_SPLIT_GUID,
                           &column_index_to_column(view,
                                                   select_after_refresh(&transaction_register)));
        refresh_account_registers(None, Some(&transaction_register.guid), globals);
    }
}

fn reverse_sign(transaction_register: &TransactionRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&transaction_register.core, globals) {
        let view = &transaction_register.core.view;
        let split_guid: String = model.get_value(&iter, STORE_SPLIT_GUID).get().unwrap();
        prepare_statement!(REVERSE_SIGN_SQL, globals).execute(params![split_guid])
                                                     .unwrap();
        update_balancing_split(&transaction_register, &split_guid, globals);
        refresh_transaction_registers(&TransactionChanged, &transaction_register.guid, globals);
        select_row_by_guid(view,
                           &split_guid,
                           STORE_SPLIT_GUID,
                           &column_index_to_column(view,
                                                   select_after_refresh(&transaction_register)));
        refresh_account_registers(None, Some(&transaction_register.guid), globals);
    }
}

fn copy_split_account_path_to_clipboard(transaction_register: &TransactionRegister,
                                        globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&transaction_register.core, globals) {
        let split_account_guid: String = model.get_value(&iter, STORE_ACCOUNT_GUID).get().unwrap();
        let full_split_account_path = guid_to_path(prepare_statement!(GUID_TO_PATH_SQL, globals),
                                                   split_account_guid.as_str());
        Clipboard::get(&Atom::intern("CLIPBOARD")).set_text(full_split_account_path.as_str());
    }
}

fn copy_split_account(transaction_register: &TransactionRegister, globals: &Globals) {
    if let Some((model, iter)) = get_selection_info(&transaction_register.core, globals) {
        let account_guid = model.get_value(&iter, STORE_ACCOUNT_GUID).get().unwrap();
        globals.account_copy_buffer.replace(Some(account_guid));
    }
}

fn paste_split_account(transaction_register: &TransactionRegister, globals: &Globals) {
    // Is there anything in the copy buffer?
    if let Some(account_copy_buffer) = (*globals.account_copy_buffer.borrow()).as_ref() {
        if let Some((model, iter)) = get_selection_info(&transaction_register.core, &globals) {
            let split_guid: String = model.get_value(&iter, STORE_SPLIT_GUID).get().unwrap();
            prepare_statement!(PASTE_ACCOUNT_GUID_SQL, globals).execute(&[account_copy_buffer,
                                                                          &split_guid])
                                                               .unwrap();
            refresh_transaction_registers(&SplitEdited, &split_guid, globals);
            refresh_account_registers(None, Some(&transaction_register.guid), globals);
        }
    } else {
        display_message_dialog("Cannot paste a split account without first doing a copy",
                               globals);
    }
}

fn display_split_account_register(transaction_register: &TransactionRegister,
                                  globals: &Rc<Globals>) {
    if let Some((model, iter)) = get_selection_info(&transaction_register.core, &globals) {
        let account_guid: String = model.get_value(&iter, STORE_ACCOUNT_GUID).get().unwrap();
        let marketable = inherited_p(prepare_statement!(INHERITED_P_SQL, globals),
                                     &account_guid,
                                     ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE);
        let path = guid_to_path(prepare_statement!(GUID_TO_PATH_SQL, globals), &account_guid);
        create_account_register(account_guid, marketable, path.as_str(), &globals);
    };
}

// Called when 'toggled' is signalled for R or T column
fn flag_bit_toggled(renderer: &CellRendererToggle, path: &TreePath,
                    transaction_register: &TransactionRegister, sql: &str,
                    store_column_index: i32, globals: &Globals) {
    let store = &transaction_register.store;
    let split_guid: String = get_string_column_via_path(store, path, STORE_SPLIT_GUID);

    // Update the database
    prepare_statement!(sql, globals).execute(params![split_guid])
                                    .unwrap();
    // Update the model and view
    update_boolean_column_via_path(store, path, !renderer.get_active(), store_column_index);

    // And refresh the transaction registers -- there might be one open for this transaction
    refresh_account_registers(None, Some(&transaction_register.guid), globals);
}

fn update_balancing_split(transaction_register: &TransactionRegister, split_guid: &str,
                          globals: &Globals) {
    // The idea here is to check if the transaction identified by the supplied guid has exactly 2 splits and the account of the split
    // other than the one identified by the split guid is not marketable.
    // The reason for avoiding marketable accounts is that if we change the value of the split, what do we do about the quantity?
    // Too complicated. Let the user balance the transaction manually.
    // If the criteria are satisfied, then set the value of the 'other' split to the negative of the supplied split, so that the transaction is now balanced.
    let nrows =
        prepare_statement!(SPLIT_COUNT_SQL, globals).query_row(params![transaction_register.guid],
                                                               get_result!(i32))
                                                    .unwrap();
    // Update the value of the balancing split if there are just two splits and the 'other' split is not marketable
    if nrows == 2 {
        // Update the balancing split if its account is not marketable or if it is and a money-market fund
        let (balancing_split_guid, account_guid) =
            prepare_statement!(GET_BALANCING_SPLIT_GUIDS_SQL, globals)
                .query_row(
                    params![transaction_register.guid, split_guid],
                    get_result!(string_string),
                )
                .unwrap();
        let marketable_p = inherited_p(prepare_statement!(INHERITED_P_SQL, globals),
                                       &account_guid,
                                       ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE);
        let update_balancing_money_market_split_stmt =
            prepare_statement!(UPDATE_BALANCING_MONEY_MARKET_SPLIT_SQL, globals);
        let update_balancing_split_value_stmt =
            prepare_statement!(UPDATE_BALANCING_SPLIT_VALUE_SQL, globals);
        let maybe_stmt: Option<&mut Statement> = if marketable_p {
            if money_market_p(&balancing_split_guid, globals) {
                Some(update_balancing_money_market_split_stmt)
            } else {
                None
            }
        } else {
            Some(update_balancing_split_value_stmt)
        };
        if let Some(stmt) = maybe_stmt {
            stmt.execute(params![transaction_register.guid, split_guid])
                .unwrap();
        }
    }
}

fn money_market_p(split_guid: &str, globals: &Globals) -> bool {
    if let Ok(flags) =
        prepare_statement!(MONEY_MARKET_P_SQL, globals).query_row(&[split_guid], get_result!(i32))
    {
        (flags & COMMODITY_FLAG_MONEY_MARKET_FUND) == 1
    } else {
        false
    }
}

fn sanitize(v: &str) -> String {
    let mut result = String::from("");
    for c in v.chars() {
        if c != '$' && c != ',' {
            result.push(c);
        }
    }
    result
}

// Called when 'edited' is signalled for value column of a transaction register for an non-marketable account
fn value_edited(path: &TreePath, new_value_expression: &str,
                transaction_register: &TransactionRegister, globals: &Globals) {
    let view = &transaction_register.core.view;
    let model = view.get_model().unwrap();
    let iter = model.get_iter(path).unwrap();
    let split_guid: String = model.get_value(&iter, STORE_SPLIT_GUID).get().unwrap();
    let split_account_guid: String = model.get_value(&iter, STORE_ACCOUNT_GUID).get().unwrap();
    let split_shares_p = inherited_p(prepare_statement!(INHERITED_P_SQL, globals),
                                     &split_account_guid,
                                     ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE);
    let new_value: f64 = if let Some(new_value) =
        evaluate_expression(sanitize(new_value_expression).as_str(), globals)
    {
        new_value
    } else {
        display_message_dialog("Invalid expression when editing the value field of a split",
                               globals);
        return;
    };

    // Is the split account marketable?
    if split_shares_p {
        // Is it a money market account? If so, we can process, because we know that quantity = value.
        if money_market_p(&split_guid, globals) {
            prepare_statement!(UPDATE_MONEY_MARKET_VALUE_QUANTITY_SQL, globals)
                .execute(params![new_value, split_guid])
                .unwrap();
        } else {
            let full_account_name: String = model.get_value(&iter, STORE_FULL_ACCOUNT_NAME)
                                                 .get()
                                                 .unwrap();
            display_message_dialog(
                                   format!(
                "The account of the split you are editing ({}) is a marketable security.
Because the transaction register was displayed from an account register for a non-marketable \
                 security,
there are no share-quantity and price columns and therefore the split value cannot be edited from \
                 this transaction register. Use
the 'Display split account register (Ctrl-s) command to obtain an account register for the split \
                 account. From there, you can display a transaction register from which you will \
                 be able to
edit this split.",
                full_account_name
            ).as_str(),
                                   globals,
            );
            return;
        }
    } else {
        prepare_statement!(UPDATE_VALUE_SQL, globals).execute(params![new_value, split_guid])
                                                     .unwrap();
    }
    update_balancing_split(&transaction_register, &split_guid, globals);
    refresh_transaction_registers(&SplitEdited, &split_guid, globals);
    select_row_by_guid(view,
                       &split_guid,
                       STORE_SPLIT_GUID,
                       &column_index_to_column(view, VIEW_VALUE));
    refresh_account_registers(None, Some(&transaction_register.guid), globals);
}

fn shares_value_edited(path: &TreePath, new_value_expression: &str,
                       transaction_register: &TransactionRegister, globals: &Globals) {
    let view = &transaction_register.core.view;
    let model: TreeModel = view.get_model().unwrap();
    let split_guid = get_string_column_via_path(&model, path, STORE_SPLIT_GUID);
    let new_value: f64 = if let Some(new_value) =
        evaluate_expression(sanitize(new_value_expression).as_str(), globals)
    {
        new_value
    } else {
        display_message_dialog("Invalid expression when editing the value field of a split",
                               globals);
        return;
    };
    // Is this a money-market fund?
    if money_market_p(&split_guid, globals) {
        prepare_statement!(UPDATE_MONEY_MARKET_VALUE_QUANTITY_SQL, globals)
            .execute(params![new_value, split_guid])
            .unwrap();
    } else {
        prepare_statement!(UPDATE_VALUE_SQL, globals).execute(params![new_value, split_guid])
                                                     .unwrap();
    };

    update_balancing_split(&transaction_register, &split_guid, globals);
    refresh_transaction_registers(&SplitEdited, &split_guid, globals);
    select_row_by_guid(view,
                       &split_guid,
                       STORE_SPLIT_GUID,
                       &column_index_to_column(view, SHARES_VIEW_VALUE));
    refresh_account_registers(None, Some(&transaction_register.guid), globals);
}

fn price_edited(path: &TreePath, new_price_expression: &str,
                transaction_register: &TransactionRegister, globals: &Globals) {
    let view = &transaction_register.core.view;
    let model = view.get_model().unwrap();
    let split_guid = get_string_column_via_path(&model, path, STORE_SPLIT_GUID);
    let new_price: f64 = if let Some(new_price) =
        evaluate_expression(sanitize(new_price_expression).as_str(), globals)
    {
        new_price
    } else {
        display_message_dialog("Invalid expression when editing the price field of a split",
                               globals);
        return;
    };

    // Here we have an issue, because price is not stored in the database. It is always computed as value/quantity. There are several cases to deal with here:
    // 1. The quantity and value fields are either zero/null. In this case, arbitrarily set the value to the desired price and set quantity to 1.
    // 2. Either (a) quantity or (b) value, but not both, is zero/null. In that case, set the zero/null field so that the desired price will be obtained.
    // 3. Both quantity and value are not zero/null. In this case, the user will have to be asked which of the two she wants to change to obtain the desired price.
    // The first step is to test the zero-/null-ness of both quantity and value, so we can determine which case we are dealing with.
    let (quantity_nullzero_ness, value_nullzero_ness) =
        prepare_statement!(PRICE_EDITED_NULL_CHECK_SQL, globals).query_row(params![split_guid],
                                                                           get_result!(bool_bool))
                                                                .unwrap();
    let case_1_stmt = prepare_statement!("update splits set value=?1, quantity = 1.0 where guid \
                                          = ?2",
                                         globals);
    let case_2a_stmt = prepare_statement!("update splits set quantity=value/(?1) where guid = ?2",
                                          globals);
    let case_2b_stmt = prepare_statement!("update splits set value=quantity*(?1) where guid = ?2",
                                          globals);
    let case_3a_stmt = prepare_statement!("update splits set value=quantity*(?1) where guid = ?2",
                                          globals);
    let case_3b_stmt = prepare_statement!("update splits set quantity=ifnull(value/(?1),0) where \
                                           guid = ?2",
                                          globals);
    let stmt = if quantity_nullzero_ness && value_nullzero_ness {
        // Case 1.
        case_1_stmt
    } else if quantity_nullzero_ness {
        // Case 2a. Since price = value/quantity we must set quantity = value/price. Value is already scaled by max denom, so the result will be scaled as well.
        case_2a_stmt
    } else if value_nullzero_ness {
        // Case 2b. Since price = value/quantity we must set value = quantity*price. quantity is already scaled by max denom, so the result will be scaled as well.
        case_2b_stmt
    } else {
        // Case 3. Neither are zero. We have to ask the user which one to change.
        const VALUE: ResponseType = ResponseType::Other(0);
        const QUANTITY: ResponseType = ResponseType::Other(1);
        let dialog = Dialog::new_with_buttons(Some("Choose field to re-compute"),
                                              Some(&transaction_register.core.window),
                                              DialogFlags::MODAL,
                                              &[("Value", VALUE), ("Shares", QUANTITY)]);
        let content_area = dialog.get_content_area();
        content_area.add(&Label::new(Some("Price equals Value / Shares.\nUnlike Shares and \
                                           Value, Price is not stored directly in the NewCash \
                                           database; it is computed as Value / Shares.\nYou \
                                           have requested a change in price, which will be \
                                           accomplished by changing Value or Shares.\nPlease \
                                           choose which one you wish to change to achieve the \
                                           desired Price.\n")));
        dialog.show_all();
        let result = dialog.run();
        dialog.destroy();
        match result {
            VALUE => case_3a_stmt,
            QUANTITY => case_3b_stmt,
            _ => return, // User probably changed her mind and hit escape. Do nothing.
        }
    };

    stmt.execute(params![new_price, split_guid]).unwrap();
    update_balancing_split(&transaction_register, &split_guid, globals);
    refresh_transaction_registers(&SplitEdited, &split_guid, globals);
    select_row_by_guid(view,
                       &split_guid,
                       STORE_SPLIT_GUID,
                       &column_index_to_column(view, SHARES_VIEW_PRICE));
    refresh_account_registers(None, Some(&transaction_register.guid), globals);
}

// Called when quantity is edited
fn quantity_edited(path: &TreePath, new_quantity_expression: &str,
                   transaction_register: &TransactionRegister, globals: &Globals) {
    let view = &transaction_register.core.view;
    let model = view.get_model().unwrap();
    let split_guid = get_string_column_via_path(&model, path, STORE_SPLIT_GUID);
    let new_quantity: f64 = if let Some(new_quantity) =
        evaluate_expression(sanitize(new_quantity_expression).as_str(), globals)
    {
        new_quantity
    } else {
        display_message_dialog("Invalid expression when editing the quantity field of a split",
                               globals);
        return;
    };
    if money_market_p(&split_guid, globals) {
        prepare_statement!(UPDATE_MONEY_MARKET_VALUE_QUANTITY_SQL, globals)
            .execute(params![new_quantity, split_guid])
            .unwrap();
    } else {
        prepare_statement!(UPDATE_QUANTITY_SQL, globals).execute(params![new_quantity, split_guid])
                                                        .unwrap();
    };
    update_balancing_split(transaction_register, &split_guid, globals);
    refresh_transaction_registers(&SplitEdited, &split_guid, globals);
    select_row_by_guid(view,
                       &split_guid,
                       STORE_SPLIT_GUID,
                       &column_index_to_column(view, SHARES_VIEW_QUANTITY));
    refresh_account_registers(None, None, globals);
}

// This routine gets called when something has changed -- an account name, a transaction, or a split -- that
// might affect the information displayed in a transaction register. The routine takes a reason argument that
// distinguishes among these three situations, and a guid that identifies the account, transaction or split that
// changed. The store for transaction registers, each row of which represents a split, has a column for
// the account guid, the transaction guid and the split guid, so this routine can identify, by searching, those
// transaction registers that are affected by the change made by the caller. Those transactions registers that
// are affected are refreshed; the rest are left as they are.
pub fn refresh_transaction_registers(reason: &WhatChanged, guid: &str, globals: &Globals) {
    fn refresh_transaction_register(transaction_register: &TransactionRegister,
                                    reason: &WhatChanged, guid: &str, globals: &Globals) {
        let view = &transaction_register.core.view;
        let column_index: i32 = match reason {
            AccountNameChanged => STORE_ACCOUNT_GUID,
            TransactionChanged => STORE_TRANSACTION_GUID,
            SplitEdited => STORE_SPLIT_GUID,
        };
        let model: TreeModel = view.get_model().unwrap();
        let iter = model.get_iter_first().unwrap();
        loop {
            let stored_guid: String = model.get_value(&iter, column_index).get().unwrap();
            if stored_guid == guid {
                // This register needs refreshing
                // Clear the store
                transaction_register.store.clear();
                populate_transaction_register_store(&transaction_register, globals);
                select_row_by_guid(view,
                                   guid,
                                   column_index,
                                   &column_index_to_column(view, VIEW_MEMO));
                return;
            }
            if !model.iter_next(&iter) {
                return;
            }
        }
    }
    for transaction_register in globals.transaction_registers.borrow().values() {
        refresh_transaction_register(transaction_register, reason, guid, globals);
    }
}

// Called when text field is edited
fn memo_edited(path: &TreePath, new_memo: &str, transaction_register: &TransactionRegister,
               globals: &Globals) {
    let model: TreeModel = transaction_register.core.view.get_model().unwrap();
    let split_guid: String = get_string_column_via_path(&model, path, STORE_SPLIT_GUID);

    // Update the database
    let temp = new_memo.to_string();
    prepare_statement!(UPDATE_MEMO_SQL, globals).execute(params![temp, split_guid])
                                                .unwrap();
    // Write new value to store
    update_string_column_via_path(&transaction_register.store, path, new_memo, STORE_MEMO);

    refresh_transaction_registers(&SplitEdited, &transaction_register.guid, &globals);
}

fn populate_transaction_register_store(transaction_register: &TransactionRegister,
                                       globals: &Globals) {
    let mut balance: f64 = 0.;
    let store = &transaction_register.store;
    // Set up the query that fetches the splits to produce the transaction register.
    if transaction_register.account_register.shares_p {
        let stmt = prepare_statement!(MARKETABLE_TRANSACTION_REGISTER_SQL, globals);
        let row_iter =
            stmt.query_map(params![transaction_register.guid],
                           |row| -> Result<(String, String, String, i32, f64, f64),
                                      rusqlite::Error> {
                               Ok((row.get(QUERY_ACCOUNT_GUID).unwrap(),
                                   row.get(QUERY_SPLIT_GUID).unwrap(),
                                   row.get(QUERY_MEMO).unwrap(),
                                   row.get(QUERY_FLAGS).unwrap(),
                                   row.get(QUERY_VALUE).unwrap(),
                                   row.get(QUERY_QUANTITY).unwrap()))
                           })
                .unwrap();
        for wrapped_result in row_iter {
            let (account_guid, split_guid, memo, flags, value, quantity) = wrapped_result.unwrap();
            let full_account_path =
                guid_to_path(prepare_statement!(GUID_TO_PATH_SQL, globals), &account_guid);

            // Append an empty row to the list store. Iter will point to the new row
            let iter = store.append();
            balance += value;
            let split_adjusted_quantity = quantity * get_split_factor(&split_guid, globals);
            let value_string = format!("{:.*}", 2, value);
            let balance_string = format!("{:.*}", 2, balance);
            let (price_string, quantity_string) = if split_adjusted_quantity != 0. {
                (format!("{:.*}", 4, value / split_adjusted_quantity),
                 format!("{:.*}", 4, split_adjusted_quantity))
            } else {
                (String::from(""), String::from(""))
            };
            let reconciled_p: bool = (flags & SPLIT_FLAG_RECONCILED) != 0;
            let transfer_p: bool = (flags & SPLIT_FLAG_TRANSFER) != 0;
            // add data
            store.set(&iter,
                      &[STORE_ACCOUNT_GUID as u32,
                        STORE_SPLIT_GUID as u32,
                        STORE_TRANSACTION_GUID as u32,
                        STORE_FULL_ACCOUNT_NAME as u32,
                        STORE_MEMO as u32,
                        STORE_R as u32,
                        STORE_VALUE as u32,
                        STORE_BALANCE as u32,
                        STORE_QUANTITY as u32,
                        STORE_PRICE as u32,
                        STORE_T as u32],
                      &[&account_guid,
                        &split_guid,
                        &transaction_register.guid,
                        &full_account_path,
                        &memo,
                        &reconciled_p,
                        &value_string,
                        &balance_string,
                        &quantity_string,
                        &price_string,
                        &transfer_p]);
        }
    } else {
        let stmt = prepare_statement!(NON_MARKETABLE_TRANSACTION_REGISTER_SQL, globals);
        let row_iter =
            stmt.query_map(params![transaction_register.guid],
                           |row| -> Result<(String, String, String, i32, f64), rusqlite::Error> {
                               Ok((row.get(QUERY_ACCOUNT_GUID).unwrap(),
                                   row.get(QUERY_SPLIT_GUID).unwrap(),
                                   row.get(QUERY_MEMO).unwrap(),
                                   row.get(QUERY_FLAGS).unwrap(),
                                   row.get(QUERY_VALUE).unwrap()))
                           })
                .unwrap();

        for wrapped_result in row_iter {
            let (account_guid, split_guid, memo, flags, value) = wrapped_result.unwrap();
            let full_account_path =
                guid_to_path(prepare_statement!(GUID_TO_PATH_SQL, globals), &account_guid);

            // Append an empty row to the list store. Iter will point to the new row
            let iter = store.append();

            balance += value;
            let value_string = format!("{:.*}", 2, value);
            let balance_string = format!("{:.*}", 2, balance);
            let reconciled_p: bool = (flags & SPLIT_FLAG_RECONCILED) != 0;
            // add data
            store.set(&iter,
                      &[STORE_ACCOUNT_GUID as u32,
                        STORE_SPLIT_GUID as u32,
                        STORE_TRANSACTION_GUID as u32,
                        STORE_FULL_ACCOUNT_NAME as u32,
                        STORE_MEMO as u32,
                        STORE_R as u32,
                        STORE_VALUE as u32,
                        STORE_BALANCE as u32],
                      &[&account_guid,
                        &split_guid,
                        &transaction_register.guid,
                        &full_account_path,
                        &memo,
                        &reconciled_p,
                        &value_string,
                        &balance_string]);
        }
    }
}

fn create_transaction_store(shares_p: bool) -> ListStore {
    if shares_p {
        ListStore::new(&[Type::String, // account guid
                         Type::String, // split guid
                         Type::String, // transaction guid
                         Type::String, // account name
                         Type::String, // memo
                         Type::Bool,   // R
                         Type::String, // value
                         Type::String, // balance
                         Type::String, // quantity
                         Type::String, // price
                         Type::Bool    /* T */])
    } else {
        ListStore::new(&[Type::String, // account guid
                         Type::String, // split guid
                         Type::String, // transaction guid
                         Type::String, // account name
                         Type::String, // memo
                         Type::Bool,   // R
                         Type::String, // value
                         Type::String  /* balance */])
    }
}

// Procedure to create transaction register
pub fn create_transaction_register(transaction_guid: String, description: String, date: &str,
                                   account_register: &Rc<AccountRegister>, globals: Rc<Globals>) {
    // Check to see if there is already a register open for this transaction
    if globals.transaction_registers
              .borrow()
              .contains_key(&transaction_guid)
    {
        display_message_dialog(&format!("A transaction register already exists for {}",
                                        description),
                               &globals);
    } else {
        let transaction_register =
            Rc::new(TransactionRegister { account_register: account_register.clone(),
                                          core: RegisterCore { view: TreeView::new(),
                                                               window:
                                                                   Window::new(WindowType::Toplevel) },
                                          description,
                                          guid: transaction_guid,
                                          store:
                                              create_transaction_store(account_register.shares_p) });

        // Record the the descriptor in the transaction_registers hashtable
        globals.transaction_registers
               .borrow_mut()
               .insert(transaction_register.guid.clone(),
                       transaction_register.clone());

        // Unwrap optional entries used repeatedly below
        let view = &transaction_register.core.view;
        let window = &transaction_register.core.window;
        let store = &transaction_register.store;

        // Hook up store to the view
        view.set_model(Some(store));

        // Populate the model/store
        populate_transaction_register_store(&transaction_register, &globals);

        // Column setup
        // Account
        {
            let renderer = CellRendererText::new();
            // Add column to the view
            let column: TreeViewColumn =
                create_tree_view_text_column(&renderer, "Account", STORE_FULL_ACCOUNT_NAME);
            view.insert_column(&column, VIEW_FULL_ACCOUNT_NAME);
            column.set_expand(true);
        }
        // Memo
        {
            let renderer = CellRendererText::new();
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            renderer.connect_edited(move |_, path, new_memo| {
                        memo_edited(&path,
                                    new_memo,
                                    &(*closure_transaction_register),
                                    &closure_globals)
                    });
            renderer.set_property_editable(true);
            // Add column to the view
            let column: TreeViewColumn =
                create_tree_view_text_column(&renderer, "Memo", STORE_MEMO);
            view.insert_column(&column, VIEW_MEMO);
            column.set_expand(true);
        }

        // R
        {
            let renderer = CellRendererToggle::new();
            let globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            renderer.connect_toggled(move |closure_renderer, path| {
                        flag_bit_toggled(closure_renderer,
                                         &path,
                                         &closure_transaction_register,
                                         TOGGLE_SPLIT_R_FLAG_SQL,
                                         STORE_R,
                                         &globals);
                    });
            renderer.set_activatable(true);
            // Add column to the view
            let column: TreeViewColumn = create_tree_view_toggle_column(&renderer, "R", STORE_R);
            view.insert_column(&column, VIEW_R);
        }

        if transaction_register.account_register.shares_p {
            // Transfer
            {
                let renderer = CellRendererToggle::new();
                let globals = globals.clone();
                let closure_transaction_register = transaction_register.clone();
                renderer.connect_toggled(move |closure_renderer, path| {
                            flag_bit_toggled(closure_renderer,
                                             &path,
                                             &closure_transaction_register,
                                             TOGGLE_SPLIT_T_FLAG_SQL,
                                             STORE_T,
                                             &globals);
                        });
                renderer.set_activatable(true);
                // Add column to the view
                let column: TreeViewColumn =
                    create_tree_view_toggle_column(&renderer, "T", STORE_T);
                view.insert_column(&column, SHARES_VIEW_T);
            }
            // Quantity
            {
                let renderer = CellRendererText::new();
                let globals = globals.clone();
                let closure_transaction_register = transaction_register.clone();
                renderer.connect_edited(move |_, path, new_quantity| {
                            quantity_edited(&path,
                                            &new_quantity,
                                            &closure_transaction_register,
                                            &globals)
                        });
                renderer.set_property_editable(true);
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Shares", STORE_QUANTITY);
                view.insert_column(&column, SHARES_VIEW_QUANTITY);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            // Price
            {
                let renderer = CellRendererText::new();
                let globals = globals.clone();
                let closure_transaction_register = transaction_register.clone();
                renderer.connect_edited(move |_, path, new_price| {
                            price_edited(&path,
                                         &new_price,
                                         &(*closure_transaction_register),
                                         &globals)
                        });
                renderer.set_property_editable(true);
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Price", STORE_PRICE);
                view.insert_column(&column, SHARES_VIEW_PRICE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            // Value
            {
                let renderer = CellRendererText::new();
                let globals = globals.clone();
                let closure_transaction_register = transaction_register.clone();
                renderer.connect_edited(move |_, path, new_factor| {
                            shares_value_edited(&path,
                                                &new_factor,
                                                &(*closure_transaction_register),
                                                &globals)
                        });
                renderer.set_property_editable(true);
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Value", STORE_VALUE);
                view.insert_column(&column, SHARES_VIEW_VALUE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            // Balance
            {
                let renderer = CellRendererText::new();
                // Add column to the view
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Balance", STORE_BALANCE);
                view.insert_column(&column, SHARES_VIEW_VALUE_BALANCE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
        } else {
            // Value
            {
                let renderer = CellRendererText::new();
                let globals = globals.clone();
                let closure_transaction_register = transaction_register.clone();
                renderer.connect_edited(move |_, path, new_factor| {
                            value_edited(&path,
                                         &new_factor,
                                         &(*closure_transaction_register),
                                         &globals)
                        });
                renderer.set_property_editable(true);
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Value", STORE_VALUE);
                view.insert_column(&column, VIEW_VALUE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
            // Balance
            {
                let renderer = CellRendererText::new();
                // Add column to the view
                let column: TreeViewColumn =
                    create_tree_view_text_column(&renderer, "Balance", STORE_BALANCE);
                view.insert_column(&column, VIEW_VALUE_BALANCE);
                // Right-justify the value column header
                column.set_alignment(1.0);
                // Make renderer right-justify the data
                renderer.set_alignment(1.0, 0.5);
            }
        }

        // Set up to handle mouse button press events
        // Build the top-level popup menu
        let transaction_register_menu = Menu::new();
        {
            let transaction_register_menu_item = MenuItem::new_with_label("New split (ctrl-n)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    new_split(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Duplicate selected split (calendar) (Ctrl-d)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    duplicate_split(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Delete selected split (Ctrl-Shift-d)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    delete_split(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Balance transaction (Ctrl-b)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    balance_transaction(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Reverse sign of value (Ctrl-r)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    reverse_sign(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Copy split account path to system clipboard (Ctrl-c)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    copy_split_account_path_to_clipboard(
                        &closure_transaction_register,
                        &closure_globals,
                    );
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Copy split account to Newcash clipboard (Alt-c)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    copy_split_account(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Paste split account from Newcash clipboard (Alt-v)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    paste_split_account(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }
        {
            let transaction_register_menu_item =
                MenuItem::new_with_label("Display split account register (Ctrl-s)");
            let closure_globals = globals.clone();
            let closure_transaction_register = transaction_register.clone();
            transaction_register_menu_item.connect_activate(
                move |_transaction_register_menu_item: &MenuItem| {
                    display_split_account_register(&closure_transaction_register, &closure_globals);
                },
            );
            transaction_register_menu.append(&transaction_register_menu_item);
        }

        view.connect_button_press_event(move |_view: &TreeView, event_button: &EventButton| {
                // single click and right button pressed?
                if (event_button.get_event_type() == ButtonPress)
                   && (event_button.get_button() == 3)
                {
                    transaction_register_menu.show_all();
                    transaction_register_menu.popup_easy(3, event_button.get_time());
                    Inhibit(true) // we handled this
                } else {
                    Inhibit(false) // we did not handle this
                }
            });

        // Connect to signal for key press events
        let globals_key_press_event = globals.clone();
        let transaction_register_key_press_event = transaction_register.clone();
        view.connect_key_press_event(move |_accounts_view: &TreeView, event_key: &EventKey| {
                let masked_state: u32 =
                    event_key.get_state().bits() & globals_key_press_event.modifiers.bits();
                // Ctrl key pressed?
                if masked_state == ModifierType::CONTROL_MASK.bits() {
                    match event_key.get_keyval() {
                        key::n => {
                            new_split(&transaction_register_key_press_event,
                                      &globals_key_press_event);
                            Inhibit(true)
                        }
                        key::d => {
                            duplicate_split(&transaction_register_key_press_event,
                                            &globals_key_press_event);
                            Inhibit(true)
                        }
                        key::b => {
                            balance_transaction(&transaction_register_key_press_event,
                                                &globals_key_press_event);
                            Inhibit(true)
                        }
                        key::r => {
                            reverse_sign(&transaction_register_key_press_event,
                                         &globals_key_press_event);
                            Inhibit(true)
                        }
                        key::s => {
                            display_split_account_register(&transaction_register_key_press_event,
                                                           &globals_key_press_event);
                            Inhibit(true)
                        }
                        key::c => {
                            copy_split_account_path_to_clipboard(
                            &transaction_register_key_press_event,
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
                            delete_split(&transaction_register_key_press_event,
                                         &globals_key_press_event);
                            Inhibit(true)
                        }
                        // Indicate we didn't handle the event
                        _ => Inhibit(false),
                    }
                } else if masked_state == ModifierType::MOD1_MASK.bits() {
                    match event_key.get_keyval() {
                        key::c => {
                            copy_split_account(&transaction_register_key_press_event,
                                               &globals_key_press_event);
                            Inhibit(true)
                        }
                        key::v => {
                            paste_split_account(&transaction_register_key_press_event,
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

        // Grid lines for readability
        view.set_grid_lines(Both);

        window.add(view);

        // Set window title to the transaction date and description
        window.set_title(format!("{} {}", date, transaction_register.description).as_str());

        // Handle deletion of the transaction window
        let globals_delete_event = globals.clone();
        let transaction_register_delete_event = transaction_register.clone();
        window.connect_delete_event(move |_, _| {
                  let mut stmt = globals_delete_event.db
                                                     .prepare(CHECK_TRANSACTION_BALANCE_SQL)
                                                     .unwrap();
                  let balance: f64 =
                      stmt.query_row(params![(&*transaction_register_delete_event.guid)],
                                     get_result!(f64))
                          .unwrap();
                  if balance.abs() > EPSILON {
                      display_message_dialog("Transaction final balance must be zero", &globals);
                      Inhibit(true)
                  } else if globals_delete_event.transaction_registers
                                                .borrow_mut()
                                                .remove(&transaction_register_delete_event.guid)
                                                .is_some()
                  {
                      Inhibit(false)
                  } else {
                      panic!("Transaction register deleted, but not found in \
                              transaction_registers hash table")
                  }
              });

        // Set the view's selection mode
        view.get_selection().set_mode(Browse);

        window.show_all();
        select_first_row(view,
                         &column_index_to_column(view,
                                                 select_after_refresh(&transaction_register)));
    }
}
