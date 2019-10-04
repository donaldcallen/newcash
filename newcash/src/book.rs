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

extern crate gtk;
use gdk::Atom;
use gtk::{
    CheckButton, Clipboard, ComboBoxExt, ComboBoxExtManual, ComboBoxText, ComboBoxTextExt,
    ContainerExt, Dialog, DialogExt, DialogFlags, EditableSignals, Entry, EntryBuffer, EntryExt,
    Grid, GtkWindowExt, ResponseType, ToggleButtonExt, TreeIter, TreeModelExt, TreePath,
    TreeSelectionExt, TreeStoreExt, TreeStoreExtManual, TreeViewExt, WidgetExt,
};

use constants::WhatChanged::AccountNameChanged;
use constants::{
    CommodityEditing, Globals, ACCOUNT_TREE_STORE_FLAGS, ACCOUNT_TREE_STORE_GUID,
    ACCOUNT_TREE_STORE_NAME,
};
use queries::{
    ACCOUNT_CHILD_ALL_SQL, ACCOUNT_CHILD_NOT_HIDDEN_SQL, ACCOUNT_INFORMATION_SQL,
    COMMODITY_INFO_SQL, DELETE_ACCOUNT_SPLIT_CHECK_STR, DELETE_ACCOUNT_SQL, DUPLICATE_CHECK_SQL,
    GET_COMMODITY_GUID_SQL, NEW_ACCOUNT_WITHOUT_COMMODITY_SQL, NEW_ACCOUNT_WITH_COMMODITY_SQL,
    PASTE_ACCOUNT_SQL, REPARENT_ACCOUNT_SQL, UPDATE_ACCOUNT_WITHOUT_COMMODITY_SQL,
    UPDATE_ACCOUNT_WITH_COMMODITY_SQL,
};
use rusqlite::{params, Statement};
use rust_library::constants::{
    ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS, ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME,
    ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE, ACCOUNT_FLAG_DESCENDENTS_ARE_TAX_DEFERRED,
    ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK, ACCOUNT_FLAG_HIDDEN, ACCOUNT_FLAG_NOCHILDREN,
    ACCOUNT_FLAG_PERMANENT, ACCOUNT_FLAG_PLACEHOLDER,
    ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED,
};
use rust_library::queries::GUID_TO_PATH_SQL;
use rust_library::queries::{INHERITED_P_SQL, NEW_UUID_SQL};
use rust_library::{guid_to_path, inherited_p};
use std::cell::RefMut;
use std::collections::HashSet;
use std::convert::TryInto;
use std::rc::Rc;
use transaction::refresh_transaction_registers;
use utilities::{create_and_enter_dialog_item, display_message_dialog};

fn set_up_commodity_combo_box(commodity_editing: &CommodityEditing, globals: &Globals) {
    // Get the pattern string and append a wildcard to it
    let pattern_string: String = if let Some(tmp) = commodity_editing.pattern_item.get_text() {
        let s = tmp.as_str();
        format!("{}%", s)
    } else {
        "%".to_string()
    };
    let stmt = prepare_statement!(COMMODITY_INFO_SQL, globals);
    let commodity_info_iter =
        stmt.query_map(params![pattern_string],
                                                               get_result!(string_string))
                                                    .unwrap();
    // Were we provided a child_guid and no pattern? If so, try to make the child_guid's commodity the active one.
    if commodity_editing.child_guid.is_some() && (pattern_string.len() == 1) {
        if let Some(ref child_guid) = commodity_editing.child_guid {
            // Yes
            // Get the commodity guid from the child
            if let Ok(current_commodity_guid) =
                prepare_statement!(GET_COMMODITY_GUID_SQL, globals).query_row(params![child_guid],
                                                                           get_result!(string))
            {
                for (i, wrapped_commodity_info) in commodity_info_iter.enumerate() {
                    let (commodity_guid, commodity_name) = wrapped_commodity_info.unwrap();
                    commodity_editing.commodity_item
                                     .append(Some(commodity_guid.as_str()),
                                             commodity_name.as_str());
                    if commodity_guid == current_commodity_guid {
                        commodity_editing.commodity_item
                                         .set_active(Some(i.try_into().unwrap()));
                    }
                }
            } else {
                panic!("set_up_commodity_combo_box: query to obtain current commodity guid \
                        failed.");
            }
        } else {
            panic!("set_up_commodity_combo_box: should be impossibile to get here!");
        }
    } else {
        // If we get here, just populate the commodity_item and arbitrarily make the first entry the active one
        for wrapped_commodity_info in commodity_info_iter {
            let (commodity_guid, commodity_name) = wrapped_commodity_info.unwrap();
            commodity_editing.commodity_item
                             .append(Some(commodity_guid.as_str()), commodity_name.as_str());
        }
        if pattern_string.len() > 1 {
            commodity_editing.commodity_item.set_active(Some(0));
        }
    };
}

pub fn pattern_text_changed(commodity_editing: &CommodityEditing, globals:&Globals) {
    commodity_editing.commodity_item.remove_all();
    set_up_commodity_combo_box(commodity_editing, globals);
}

pub fn new_account(globals: &Rc<Globals>) {
    // Is there a row selected?
    if let Some((model, parent_iter)) = globals.accounts_view.get_selection().get_selected() {
        // Get the account's flags
        let account_flags: i32 = model.get_value(&parent_iter, ACCOUNT_TREE_STORE_FLAGS)
                                      .get()
                                      .unwrap();
        if (account_flags & ACCOUNT_FLAG_NOCHILDREN) != 0 {
            display_message_dialog("You may not create children of this account.", &globals);
        } else {
            let name_item = Entry::new_with_buffer(&EntryBuffer::new(None));
            let code_item = Entry::new_with_buffer(&EntryBuffer::new(None));
            let description_item = Entry::new_with_buffer(&EntryBuffer::new(None));
            let hidden_item = CheckButton::new();
            let placeholder_item = CheckButton::new();
            let self_and_descendents_are_tax_related_item = CheckButton::new();
            let mut descendents_are_marketable_item: Option<CheckButton> = None;
            let mut descendents_are_income_generators_item: Option<CheckButton> = None;
            let mut tax_deferred_item: Option<CheckButton> = None;
            let asset_p: bool;
            let mut marketable_p: bool = false;
            let mut income_from_commodity_p: bool = false;
            let mut income_p: bool = false;
            let mut row = 1;
            // Create the dialog box to collect all the information needed about the account we are editing, whether new or existing.
            let dialog = Dialog::new_with_buttons(Some("New Account"),
                                                  Some(&globals.accounts_window),
                                                  DialogFlags::MODAL,
                                                  &[("OK", ResponseType::Ok),
                                                    ("Cancel", ResponseType::Cancel)]);
            let content_area = dialog.get_content_area();
            // We will lay out the dialog using a grid
            let grid = Grid::new();

            // Create and place the objects in the dialog grid
            create_and_enter_dialog_item(&grid, "Name:", row, &name_item);
            row += 1;
            create_and_enter_dialog_item(&grid, "Code:", row, &code_item);
            row += 1;
            create_and_enter_dialog_item(&grid, "Description:", row, &description_item);
            row += 1;
            create_and_enter_dialog_item(&grid, "Hidden?:", row, &hidden_item);
            row += 1;
            create_and_enter_dialog_item(&grid, "Placeholder?:", row, &placeholder_item);
            row += 1;
            create_and_enter_dialog_item(&grid,
                                         "Self and descendents are tax-related?:",
                                         row,
                                         &self_and_descendents_are_tax_related_item);
            row += 1;

            // We are creating a new account. parent_iter points to the selected account, which is the parent of the new account
            // Get the parent guid
            let parent_guid: String = globals.accounts_store
                                             .get_value(&parent_iter, ACCOUNT_TREE_STORE_GUID)
                                             .get()
                                             .unwrap();
            if parent_guid != (*globals.root_account_guid) {
                // If we get here, parent is not the root.
                let commodity_editing =
                    Rc::new(CommodityEditing { child_guid: None,
                                               commodity_item: ComboBoxText::new(),
                                               pattern_item:
                                                   Entry::new_with_buffer(&EntryBuffer::new(None)) });
                asset_p = inherited_p(prepare_statement!(INHERITED_P_SQL, globals),
                                      &parent_guid,
                                      ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS);
                if asset_p {
                    // This is intended to indicate whether the new account is marketable (inherits descendents are marketable). That will only be the case if the parent inherits the descendents_are_marketable flag
                    // or the parent has that flag-bit set itself
                    let flags: i32 = globals.accounts_store
                                            .get_value(&parent_iter, ACCOUNT_TREE_STORE_FLAGS)
                                            .get()
                                            .unwrap();
                    marketable_p = inherited_p(prepare_statement!(INHERITED_P_SQL,
                                                                         globals),
                                               &parent_guid,
                                               ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE)
                                   || ((flags & ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE) != 0);
                } else {
                    income_p = inherited_p(prepare_statement!(INHERITED_P_SQL,
                                                                     globals),
                                           &parent_guid,
                                           ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME);
                    if income_p {
                        // This is the same situation as described in the comment above.
                        // For the new account to have the income-from-commodity property, the parent either has to have it or inherit it
                        let flags: i32 = globals.accounts_store
                                                .get_value(&parent_iter, ACCOUNT_TREE_STORE_FLAGS)
                                                .get()
                                                .unwrap();
                        income_from_commodity_p =
                            inherited_p(prepare_statement!(INHERITED_P_SQL,
                                                                  globals),
                                        &parent_guid,
                                        ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)
                            || ((flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK) != 0);
                    }
                }
                if asset_p && !marketable_p {
                    // This is only needed for unmarketable assets
                    let temp = CheckButton::new();
                    create_and_enter_dialog_item(&grid, "Descendents are marketable?:", row, &temp);
                    descendents_are_marketable_item = Some(temp);
                } else if income_p && !income_from_commodity_p {
                    // This is only needed for income accounts that haven't been designated as having descendents generating income
                    let temp = CheckButton::new();
                    create_and_enter_dialog_item(&grid,
                                                 "Descendents generate income from commodities?:",
                                                 row,
                                                 &temp);
                    descendents_are_income_generators_item = Some(temp);
                } else if (asset_p && marketable_p) || (income_p && income_from_commodity_p) {
                    if asset_p && marketable_p {
                        let temp = CheckButton::new();
                        create_and_enter_dialog_item(&grid,
                                                     "Descendents are tax-deferred?:",
                                                     row,
                                                     &temp);
                        row += 1;
                        tax_deferred_item = Some(temp);
                    }
                    // Handle the "changed" signal of the pattern entry.
                    // The commodity combobox will be populated  by the callback per the pattern
                    let commodity_editing_changed = commodity_editing.clone();
                    let closure_globals = globals.clone();
                    commodity_editing.pattern_item.connect_changed(move |_| {
                        pattern_text_changed(&commodity_editing_changed, &closure_globals);
                    });

                    create_and_enter_dialog_item(&grid,
                                                 "Pattern:",
                                                 row,
                                                 &commodity_editing.pattern_item);
                    row += 1;
                    create_and_enter_dialog_item(&grid,
                                                 "Commodity:",
                                                 row,
                                                 &commodity_editing.commodity_item);
                    set_up_commodity_combo_box(&commodity_editing, &globals);
                }

                content_area.add(&grid);
                dialog.show_all();

                if dialog.run() == ResponseType::Ok {
                    let temp = name_item.get_text().unwrap();
                    let name: &str = temp.as_str();
                    if prepare_statement!(DUPLICATE_CHECK_SQL, globals).query_row(params![name,
                                                                                       parent_guid],
                                                                               get_result!(string))
                                                                    .is_ok()
                    {
                        display_message_dialog("You attempted to create a new account with the \
                                                same name as an existing child of the chosen \
                                                parent account. The new account name must be \
                                                unique. Try again.",
                                               &globals);
                    } else {
                        let hidden = hidden_item.get_active();
                        let flags = (if let Some(dm) = descendents_are_marketable_item {
                            (if dm.get_active() {
                                ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE
                            } else {
                                0
                            })
                        } else {
                            0
                        }) | if let Some(dig) = descendents_are_income_generators_item {
                            (if dig.get_active() {
                                ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK
                            } else {
                                0
                            })
                        } else {
                            0
                        } | (if hidden { ACCOUNT_FLAG_HIDDEN } else { 0 })
                                    | (if placeholder_item.get_active() {
                                        ACCOUNT_FLAG_PLACEHOLDER
                                    } else {
                                        0
                                    })
                                    | if let Some(tdi) = tax_deferred_item {
                                        (if tdi.get_active() {
                                            ACCOUNT_FLAG_DESCENDENTS_ARE_TAX_DEFERRED
                                        } else {
                                            0
                                        })
                                    } else {
                                        0
                                    }
                                    | if self_and_descendents_are_tax_related_item.get_active() {
                                        ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED
                                    } else {
                                        0
                                    };

                        // Store information provided by the user into the database
                        let child_guid = prepare_statement!(NEW_UUID_SQL, globals)
                            .query_row(params![], get_result!(string))
                            .unwrap();
                        if ((asset_p && marketable_p) || (income_p && income_from_commodity_p))
                           && ((flags & ACCOUNT_FLAG_PLACEHOLDER) == 0)
                        {
                            prepare_statement!(NEW_ACCOUNT_WITH_COMMODITY_SQL, globals)
                                .execute(params![
                                    child_guid,
                                    name,
                                    parent_guid,
                                    code_item.get_text().unwrap().as_str(),
                                    description_item.get_text().unwrap().as_str(),
                                    flags,
                                    commodity_editing
                                        .commodity_item
                                        .get_active_id()
                                        .unwrap()
                                        .as_str(),
                                ])
                                .unwrap();
                        } else {
                            prepare_statement!(NEW_ACCOUNT_WITHOUT_COMMODITY_SQL, globals)
                                .execute(params![
                                    child_guid,
                                    name,
                                    parent_guid,
                                    code_item.get_text().unwrap().as_str(),
                                    description_item.get_text().unwrap().as_str(),
                                    flags,
                                ])
                                .unwrap();
                        }
                        if !hidden || *globals.show_hidden.borrow() {
                            // Insert the new row in the store only if not hidden, or we are showing hidden accounts
                            let new_account_iter = globals.accounts_store
                                                          .insert_after(Some(&parent_iter), None);
                            globals.accounts_store.set(&new_account_iter,
                                                       &[ACCOUNT_TREE_STORE_NAME as u32,
                                                         ACCOUNT_TREE_STORE_GUID as u32,
                                                         ACCOUNT_TREE_STORE_FLAGS as u32],
                                                       &[&name, &child_guid, &flags]);
                        }
                    }
                }
                dialog.destroy();
            } else {
                // The parent is the root account. Cannot create new children of the root account
                display_message_dialog("You may not create children of the Root account", &globals);
            }
        }
    } else {
        display_message_dialog("Improper selection. Cannot perform new account operation.",
                               &globals);
    }
}

pub fn edit_account(globals: &Rc<Globals>) {
    // Get the user's selection. We are editing an existing account; the selected account is the one we will edit
    if let Some((_model, iter)) = globals.accounts_view.get_selection().get_selected() {
        let name_item = Entry::new_with_buffer(&EntryBuffer::new(None));
        let code_item = Entry::new_with_buffer(&EntryBuffer::new(None));
        let description_item = Entry::new_with_buffer(&EntryBuffer::new(None));
        let hidden_item = CheckButton::new();
        let placeholder_item = CheckButton::new();
        let self_and_descendents_are_tax_related_item = CheckButton::new();
        let mut descendents_are_marketable_item: Option<CheckButton> = None;
        let mut descendents_are_income_generators_item: Option<CheckButton> = None;
        let mut tax_deferred_item: Option<CheckButton> = None;
        let asset_p: bool;
        let mut marketable_p: bool = false;
        let mut income_from_commodity_p: bool = false;
        let mut income_p: bool = false;
        let mut row = 1;

        // Create the dialog box to collect all the information needed about the account we are editing.
        let dialog = Dialog::new_with_buttons(Some("Edit Account"),
                                              Some(&globals.accounts_window),
                                              DialogFlags::MODAL,
                                              &[("OK", ResponseType::Ok),
                                                ("Cancel", ResponseType::Cancel)]);
        let content_area = dialog.get_content_area();

        // We will lay out the dialog using a grid
        let grid = Grid::new();

        // Create and place the objects in the dialog grid
        create_and_enter_dialog_item(&grid, "Name:", row, &name_item);
        row += 1;
        create_and_enter_dialog_item(&grid, "Code:", row, &code_item);
        row += 1;
        create_and_enter_dialog_item(&grid, "Description:", row, &description_item);
        row += 1;
        create_and_enter_dialog_item(&grid, "Hidden?:", row, &hidden_item);
        row += 1;
        create_and_enter_dialog_item(&grid, "Placeholder?:", row, &placeholder_item);
        row += 1;
        create_and_enter_dialog_item(&grid,
                                     "Self and descendents are tax-related?:",
                                     row,
                                     &self_and_descendents_are_tax_related_item);
        row += 1;

        if let Some(parent_iter) = globals.accounts_store.iter_parent(&iter) {
            let parent_guid: String = globals.accounts_store
                                             .get_value(&parent_iter, ACCOUNT_TREE_STORE_GUID)
                                             .get()
                                             .unwrap();
            if parent_guid == *globals.root_account_guid {
                display_message_dialog(
                                       "Editing the children of the Root account is dangerous \
                                        and is discouraged. 
                Make changes only if you are sure you know what you are doing.",
                                       &globals,
                );
            }

            // We're editing an existing account. iter points to the selected account, which is the account we will edit.
            // Get the guid of the selected account
            let commodity_editing =
                Rc::new(CommodityEditing { child_guid:
                                               globals.accounts_store
                                                      .get_value(&iter, ACCOUNT_TREE_STORE_GUID)
                                                      .get(),
                                           commodity_item: ComboBoxText::new(),
                                           pattern_item:
                                               Entry::new_with_buffer(&EntryBuffer::new(None)) });

            // Get the account information from the database
            let (name, code, description, flags) =
                prepare_statement!(ACCOUNT_INFORMATION_SQL, globals)
                    .query_row(
                        params![commodity_editing.child_guid],
                        get_result!(string_string_string_i32),
                    )
                    .unwrap();

            asset_p = inherited_p(prepare_statement!(INHERITED_P_SQL, globals),
                                  commodity_editing.child_guid.as_ref().unwrap(),
                                  ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS);
            if asset_p {
                marketable_p = inherited_p(prepare_statement!(INHERITED_P_SQL,
                                                                     globals),
                                           commodity_editing.child_guid.as_ref().unwrap(),
                                           ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE);
                if !marketable_p {
                    // This is only needed for unmarketable assets
                    let temp = CheckButton::new();
                    create_and_enter_dialog_item(&grid, "Descendents are marketable?:", row, &temp);
                    row += 1;
                    temp.set_active((flags & ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE) != 0);
                    descendents_are_marketable_item = Some(temp);

                    let temp = CheckButton::new();
                    create_and_enter_dialog_item(&grid,
                                                 "Descendents are tax-deferred?:",
                                                 row,
                                                 &temp);
                    row += 1;
                    temp.set_active((flags & ACCOUNT_FLAG_DESCENDENTS_ARE_TAX_DEFERRED) != 0);
                    tax_deferred_item = Some(temp);
                }
            } else {
                income_p = inherited_p(prepare_statement!(INHERITED_P_SQL,
                                                                 globals),
                                       commodity_editing.child_guid.as_ref().unwrap(),
                                       ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME);
                if income_p {
                    income_from_commodity_p =
                        inherited_p(prepare_statement!(INHERITED_P_SQL, globals),
                                    commodity_editing.child_guid.as_ref().unwrap(),
                                    ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK);
                    if !income_from_commodity_p {
                        // This is only needed for income accounts that haven't been designated as having descendents generating income
                        let temp = CheckButton::new();
                        create_and_enter_dialog_item(&grid,
                                                     "Descendents generate income from \
                                                      commodities?:",
                                                     row,
                                                     &temp);
                        row += 1;
                        temp.set_active((flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)
                                        != 0);
                        descendents_are_income_generators_item = Some(temp);
                    }
                }
            }

            name_item.set_text(&name);
            code_item.set_text(&code);
            description_item.set_text(&description);

            hidden_item.set_active((flags & ACCOUNT_FLAG_HIDDEN) != 0);
            placeholder_item.set_active((flags & ACCOUNT_FLAG_PLACEHOLDER) != 0);
            self_and_descendents_are_tax_related_item
                .set_active((flags & ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED) != 0);

            // Only include pattern and commodities if we are editing a marketable asset account that is not a placeholder
            if (asset_p && marketable_p)
               || (income_p && income_from_commodity_p) && (flags & ACCOUNT_FLAG_PLACEHOLDER) == 0
            {
                // Handle the "changed" signal of the pattern entry.
                // The commodity combobox will be populated  by the callback per the pattern
                let commodity_editing_changed = commodity_editing.clone();
                let closure_globals = globals.clone();
                commodity_editing.pattern_item.connect_changed(move |_| {
                                                  pattern_text_changed(&commodity_editing_changed, &closure_globals);
                                              });

                create_and_enter_dialog_item(&grid,
                                             "Pattern:",
                                             row,
                                             &commodity_editing.pattern_item);
                row += 1;
                create_and_enter_dialog_item(&grid,
                                             "Commodity:",
                                             row,
                                             &commodity_editing.commodity_item);
                set_up_commodity_combo_box(&commodity_editing, &globals);
            }
            content_area.add(&grid);
            dialog.show_all();

            if dialog.run() == ResponseType::Ok {
                let temp = name_item.get_text().unwrap();
                let name: &str = temp.as_str();
                let hidden = hidden_item.get_active();

                if &parent_guid != commodity_editing.child_guid.as_ref().unwrap() {
                    let flags = (if let Some(dm) = descendents_are_marketable_item {
                        (if dm.get_active() {
                            ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE
                        } else {
                            0
                        })
                    } else {
                        0
                    }) | if let Some(dig) = descendents_are_income_generators_item {
                        (if dig.get_active() {
                            ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK
                        } else {
                            0
                        })
                    } else {
                        0
                    } | (if hidden { ACCOUNT_FLAG_HIDDEN } else { 0 })
                                | (if placeholder_item.get_active() {
                                    ACCOUNT_FLAG_PLACEHOLDER
                                } else {
                                    0
                                })
                                | if let Some(tdi) = tax_deferred_item {
                                    (if tdi.get_active() {
                                        ACCOUNT_FLAG_DESCENDENTS_ARE_TAX_DEFERRED
                                    } else {
                                        0
                                    })
                                } else {
                                    0
                                }
                                | if self_and_descendents_are_tax_related_item.get_active() {
                                    ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED
                                } else {
                                    0
                                };

                    if ((asset_p && marketable_p) || (income_p && income_from_commodity_p))
                       && ((flags & ACCOUNT_FLAG_PLACEHOLDER) == 0)
                    {
                        prepare_statement!(UPDATE_ACCOUNT_WITH_COMMODITY_SQL, globals)
                            .execute(params![
                                name,
                                code_item.get_text().unwrap().as_str(),
                                description_item.get_text().unwrap().as_str(),
                                flags,
                                commodity_editing
                                    .commodity_item
                                    .get_active_id()
                                    .unwrap()
                                    .as_str(),
                                commodity_editing.child_guid,
                            ])
                            .unwrap();
                    } else {
                        prepare_statement!(UPDATE_ACCOUNT_WITHOUT_COMMODITY_SQL, globals)
                            .execute(params![
                                name,
                                code_item.get_text().unwrap().as_str(),
                                description_item.get_text().unwrap().as_str(),
                                flags,
                                commodity_editing.child_guid,
                            ])
                            .unwrap();
                    }
                    // The name might have changed, so have to clear the guid-to-full-path hash table, because this account
                    // might be in the middle of some paths
                    globals.guid_to_full_path.borrow_mut().clear();

                    if hidden && !(*globals.show_hidden.borrow()) {
                        // If hidden and not showing hidden, remove the row
                        globals.accounts_store.remove(&iter);
                    } else {
                        // update the store
                        globals.accounts_store.set(&iter,
                                                   &[ACCOUNT_TREE_STORE_NAME as u32,
                                                     ACCOUNT_TREE_STORE_FLAGS as u32],
                                                   &[&name, &flags]);
                    }

                    // Refresh any open transaction registers, in case they reference this account
                    refresh_transaction_registers(&AccountNameChanged,
                                                  commodity_editing.child_guid.as_ref().unwrap(),
                                                  globals);
                } else {
                    display_message_dialog("Account editing failed: you attempted to make an \
                                            account it's own parent",
                                           &globals);
                };
            };
            dialog.destroy();
        };
    } else {
        display_message_dialog("Improper selection. Cannot perform edit account operation.",
                               &globals);
    };
}

pub fn copy_account_value_to_clipboard(column: i32, error_message: &str, pathp: bool,
                                       globals: &Globals) {
    // Is there a row selected?
    if let Some((model, iter)) = globals.accounts_view.get_selection().get_selected() {
        // There is
        let column_value: String = model.get_value(&iter, column).get().unwrap();
        if pathp {
            assert_eq!(column, ACCOUNT_TREE_STORE_GUID); // if pathp is true, then column must be the guid
                                                         // This odd-looking bit of code is the way it is to avoid a run-time error. The conventional
                                                         // if let Some(path) = globals.guid_to_full_path.borrow().get(&column_value)
                                                         // in the next line won't work, because if the else clause is executed, a mutable borrow will be attempted
                                                         // of guid_to_full_path, while it is still immutably borrowed due to the if. The test of the column value
                                                         // has to be in an inner scope so that the immutable borrow is released before attempting the else.
            if if let Some(path) = globals.guid_to_full_path.borrow().get(&column_value) {
                Clipboard::get(&Atom::intern("CLIPBOARD")).set_text(path.as_str());
                false
            } else {
                true
            } {
                // The requested path isn't in the hashtable. Enter it and call the function again.
                let path: String = guid_to_path(prepare_statement!(GUID_TO_PATH_SQL,
                                                                          globals),
                                                &column_value);
                // This has to be done before the insert, because the insert requires ownership of 'path'
                Clipboard::get(&Atom::intern("CLIPBOARD")).set_text(path.as_str());
                globals.guid_to_full_path
                       .borrow_mut()
                       .insert(column_value.clone(), path);
            }
        } else {
            Clipboard::get(&Atom::intern("CLIPBOARD")).set_text(column_value.as_str());
        }
    } else {
        display_message_dialog(error_message, globals);
    };
}

pub fn copy_account_guid_to_account_copy_buffer(globals: &Globals) {
    // Is there a row selected?
    if let Some((model, iter)) = globals.accounts_view.get_selection().get_selected() {
        // Yes
        let account_guid: String = model.get_value(&iter, ACCOUNT_TREE_STORE_GUID)
                                        .get()
                                        .unwrap();
        let mut copy_buffer = globals.account_copy_buffer.borrow_mut();
        *copy_buffer = Some(account_guid);
    } else {
        display_message_dialog("Improper selection. Cannot perform requested copy of account.",
                               globals);
    };
}

pub fn reparent_account(globals: &Globals) {
    if let Some(ref parent_guid) = *globals.account_copy_buffer.borrow() {
        // The guid in the account_copy_buffer points to the account that will be the new parent
        // Get the user's selection
        if let Some((model, iter)) = globals.accounts_view.get_selection().get_selected() {
            // Get the guid of the selected account
            let account_guid: String = model.get_value(&iter, ACCOUNT_TREE_STORE_GUID)
                                            .get()
                                            .unwrap();
            prepare_statement!(REPARENT_ACCOUNT_SQL, globals).execute(params![parent_guid,
                                                                           account_guid])
                                                          .unwrap();
            // Delete from guid-to-full-path hash table
            globals.guid_to_full_path.borrow_mut().remove(&account_guid);
            refresh_accounts_window(globals);
        };
    } else {
        display_message_dialog("Before attempting to re-parent an account, you must select the \
                                new parent and do a 'Copy account' operation",
                               globals);
    };
}

fn refresh_accounts_window(globals: &Globals) {
    create_accounts_model(globals);
    // Expand root node
    let path = TreePath::new_from_string("0");
    globals.accounts_view.expand_row(&path, false);
    if *(globals.show_hidden.borrow()) {
        globals.accounts_window
               .set_title(format!("{} ({}; hidden accounts displayed)",
                                  &globals.db_path, &globals.book_name).as_str());
    } else {
        globals.accounts_window
               .set_title(format!("{} ({})", &globals.db_path, &globals.book_name).as_str());
    }

    // Enable interactive search
    globals.accounts_view.set_enable_search(true);
    globals.accounts_view
           .set_search_column(ACCOUNT_TREE_STORE_NAME);
}

pub fn toggle_show_hidden(globals: &Globals) {
    // Flip the setting in globals
    let temp = { !*globals.show_hidden.borrow() };
    globals.show_hidden.replace(temp);
    refresh_accounts_window(&globals);
}

pub fn paste_account(globals: &Globals) {
    // Is there a row selected?
    let selection = globals.accounts_view.get_selection();
    match selection.get_selected() {
        Some((model, iter)) => {
            let parent_guid: String = model.get_value(&iter, ACCOUNT_TREE_STORE_GUID)
                                           .get()
                                           .unwrap();

            // Complain if there's nothing in the copy buffer
            if let Some(ref account_copy_buffer) = *(globals.account_copy_buffer.borrow()) {
                prepare_statement!(PASTE_ACCOUNT_SQL, globals).execute(params![account_copy_buffer,
                                                                            parent_guid])
                                                           .unwrap();
                refresh_accounts_window(globals);
            } else {
                display_message_dialog("Before attempting to paste an account, you must first \
                                        copy one.",
                                       globals);
            }
        }
        None => display_message_dialog("Improper selection. Cannot perform requested paste of \
                                        account.",
                                       globals),
    };
}

// Delete account only if it is not permanent and there are no transactions and no children
pub fn delete_account(globals: &Globals) {
    // Get the user's selection
    let selection = globals.accounts_view.get_selection();
    // See if there is a valid selection
    if let Some((model, iter)) = selection.get_selected() {
        // Get the account's flags
        let account_flags: i32 = model.get_value(&iter, ACCOUNT_TREE_STORE_FLAGS)
                                      .get()
                                      .unwrap();
        if (account_flags & ACCOUNT_FLAG_PERMANENT) != 0 {
            display_message_dialog("You may not delete a permanent account.", globals);
        } else {
            // Get the guid of the selected account
            let account_guid: String = model.get_value(&iter, ACCOUNT_TREE_STORE_GUID)
                                            .get()
                                            .unwrap();
            // See if there are any splits that point to this account or any children of the account
            let split_and_child_count: i32 =
                prepare_statement!(DELETE_ACCOUNT_SPLIT_CHECK_STR, globals)
                    .query_row(params![account_guid], get_result!(i32))
                    .unwrap();
            if split_and_child_count != 0 {
                display_message_dialog(
                                       "Cannot delete account for which there are transactions \
                                        or child accounts.
To delete this account, you must first delete any transactions having
splits pointing to this account, or change the splits to point to a different account. 
If there are child accounts, those must be deleted before attempting to delete this account.",
                                       globals,
                );
            } else {
                // Delete the account
                prepare_statement!(DELETE_ACCOUNT_SQL, globals).execute(params![account_guid])
                                                            .unwrap();
                // Delete from guid-to-full-path hash table
                globals.guid_to_full_path.borrow_mut().remove(&account_guid);
                // And delete from guid_processed
                globals.guid_processed.borrow_mut().remove(&account_guid);
                // Remove from the store
                globals.accounts_store.remove(&iter);
            }
        }
    } else {
        display_message_dialog("Improper selection. Cannot perform delete operation.",
                               globals);
    }
}

pub fn add_account_tree_child_nodes(parent_guid: &str, parent_iter: &TreeIter, globals: &Globals) {
    fn process_children(parent_guid: &str, parent_iter: &TreeIter, globals: &Globals) {
        let all_stmt = prepare_statement!(ACCOUNT_CHILD_ALL_SQL, globals);
        let not_hidden_stmt = prepare_statement!(ACCOUNT_CHILD_NOT_HIDDEN_SQL, globals);
        let children_stmt: &mut Statement = if *(globals.show_hidden.borrow()) {
            all_stmt
        } else {
            not_hidden_stmt
        };

        let children_stmt_iter = children_stmt.query_map(params![parent_guid],
                                                         get_result!(string_string_i32))
                                              .unwrap();
        for wrapped_child in children_stmt_iter {
            // Have to create a row (node) for the child
            let child_iter = globals.accounts_store.append(Some(parent_iter));
            let (name, guid, flags) = wrapped_child.unwrap();
            globals.accounts_store.set(&child_iter,
                                       &[ACCOUNT_TREE_STORE_NAME as u32,
                                         ACCOUNT_TREE_STORE_GUID as u32,
                                         ACCOUNT_TREE_STORE_FLAGS as u32],
                                       &[&name, &guid, &flags]);
        }
    };

    // Have we already processed this guid?
    let mut guid_processed: RefMut<HashSet<String>> = globals.guid_processed.borrow_mut();
    if !guid_processed.contains(parent_guid) {
        // Remember that we processed it
        guid_processed.insert(parent_guid.to_string());
        process_children(parent_guid, parent_iter, globals);
    };
}

// The handling of the account tree is slightly tricky business.
// I am trying in general here to use a 'just-in-time' policy, leaving
// everything in the database until it is needed, thereby avoiding the long startup times
// of gnucash, which pulls everything into internal memory when started.
//
// The issue here is that I want to initially display the children of the root
// account, but I want them to be expandable, so the accounts window store has to be initialized
// with both the children and grandchildren of the root.
// Think of the initial setup as the response to the invisible root account being clicked.
// If one of the root's children is then clicked to expand it, *its* children will appear in the window,
// and so *its* grandchildren must be added to the store. So after initialization, clicking a node the first
// time will require that its grandchildren get added to the store.
// So the addition of grandchildren has to be conditional -- we don't want to do it multiple times, i.e. when
// its grandparent has been expanded more than once. We take care of this problem with a hash table,
// in which we record the guids of accounts that
// have already been processed by this routine.

pub fn create_accounts_model(globals: &Globals) {
    // Clear the store
    globals.accounts_store.clear();
    // Clear the hash table that records nodes already processed
    globals.guid_processed.borrow_mut().clear();
    // Create the root node
    let root_iter = globals.accounts_store.append(None);
    globals.accounts_store.set(&root_iter,
                               &[ACCOUNT_TREE_STORE_NAME as u32,
                                 ACCOUNT_TREE_STORE_GUID as u32,
                                 ACCOUNT_TREE_STORE_FLAGS as u32],
                               &[&"Root",
                                 &(*(globals.root_account_guid)),
                                 &ACCOUNT_FLAG_PLACEHOLDER]);

    // Add the children of the root node
    add_account_tree_child_nodes(&globals.root_account_guid, &root_iter, globals);

    // Must also add the grandchildren of the root. See big comment above.
    // Initialize child_iter to point to the first child of the root

    if let Some(child_iter) = globals.accounts_store.get_iter_from_string("0:0") {
        loop {
            let child_guid: String = globals.accounts_store
                                            .get_value(&child_iter, ACCOUNT_TREE_STORE_GUID)
                                            .get()
                                            .unwrap();
            add_account_tree_child_nodes(child_guid.as_str(), &child_iter, globals);
            if !globals.accounts_store.iter_next(&child_iter) {
                break;
            }
        }
    };
}
