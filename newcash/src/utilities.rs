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

use constants::{FindCommand, FindParameters, Globals, RegisterCore, DATE_ERROR_MESSAGE};
use gtk::{
    Align, ButtonsType, CellLayoutExt, CellRendererText, CellRendererToggle, ComboBoxExtManual,
    ComboBoxText, ComboBoxTextExt, ContainerExt, Dialog, DialogExt, DialogFlags, Entry,
    EntryBuffer, EntryExt, Grid, GridExt, GtkListStoreExtManual, IsA, Label, ListStore,
    MessageDialog, MessageType, ResponseType, TreeIter, TreeModel, TreeModelExt, TreePath,
    TreeSelectionExt, TreeView, TreeViewColumn, TreeViewColumnExt, TreeViewExt, Type, Widget,
    WidgetExt,
};
use regex::Regex;
use rusqlite::{params, Statement};
use std::cell::RefCell;
use std::convert::TryInto;

// expression is either a number or a numeric expression. If it's just a number, parse should give us the value.
// If not, evaluate the expression with a query
pub fn evaluate_expression(expression: &str, globals: &Globals) -> Option<f64> {
    let maybe_result = expression.parse::<f64>();
    if let Ok(result) = maybe_result {
        Some(result)
    } else {
        let sql = format!("select {}", expression);
        let mut maybe_stmt = (&(globals.db)).prepare(sql.as_str());
        if let Ok(ref mut stmt) = maybe_stmt {
            Some(stmt.query_row(params![], get_result!(f64)).unwrap())
        } else {
            None
        }
    }
}

pub fn display_message_dialog(message: &str, globals: &Globals) {
    let dialog: MessageDialog = MessageDialog::new(Some(&globals.accounts_window),
                                                   DialogFlags::MODAL,
                                                   MessageType::Info,
                                                   ButtonsType::Ok,
                                                   message);
    dialog.run();
    dialog.destroy();
}

pub fn maybe_date_increment(trimmed_new_date: &str) -> Option<i32> {
    // Is the trimmed_new_date an integer and therefore should be treated as an increment?
    if let Ok(increment) = trimmed_new_date.parse::<i32>() {
        // trimmed_new_date is an integer, now stored in increment.
        return Some(increment);
    } else if trimmed_new_date.len() == 1 {
        match &trimmed_new_date[0..1] {
            "=" => return Some(1),
            "-" => return Some(-1),
            "+" => return Some(7),
            "_" => return Some(-7),
            _ => return None,
        }
    }
    None
}

// This function checks the 'date' argument to see if it is valid ISO-8601 format
pub fn maybe_date(date: &str, globals: &Globals) -> bool {
    prepare_statement!("select julianday(?1)", globals)
        .query_row(
            params![date.to_string()],
            |row| -> Result<f64, rusqlite::Error> { row.get(0) },
        )
        .is_ok()
}

pub fn date_edited(guid: &str, increment_stmt: &mut Statement,
                   beginning_of_month_stmt: &mut Statement, end_of_month_stmt: &mut Statement,
                   today_stmt: &mut Statement, user_entry_stmt: &mut Statement, new_date: &str,
                   globals: &Globals) {
    let trimmed_new_date = new_date.trim();
    if let Some(increment) = maybe_date_increment(trimmed_new_date) {
        increment_stmt.execute(params![increment, guid]).unwrap();
    } else if trimmed_new_date.len() == 1 {
        match trimmed_new_date {
            "m" => {
                // Start of current month
                beginning_of_month_stmt.execute(params![guid]).unwrap();
            }
            "h" => {
                // End of current month
                end_of_month_stmt.execute(params![guid]).unwrap();
            }
            "t" => {
                // Today
                today_stmt.execute(params![guid]).unwrap();
            }
            _ => display_message_dialog(DATE_ERROR_MESSAGE, globals),
        }
    } else if maybe_date(trimmed_new_date, globals) {
        // Did the user provide a date?
        user_entry_stmt.execute(params![(trimmed_new_date.to_string()), guid])
                       .unwrap();
    } else {
        display_message_dialog(DATE_ERROR_MESSAGE, globals);
    }
}

pub fn get_string_column_via_path<T: TreeModelExt>(model: &T, path: &TreePath, column: i32)
                                                   -> String {
    if let Some(iter) = model.get_iter(path) {
        model.get_value(&iter, column).get().unwrap()
    } else {
        panic!("get_string_column_via_path: gtk_tree_model_get_iter failed");
    }
}

pub fn get_boolean_column_via_path<T: TreeModelExt>(model: &T, path: &TreePath, column: i32)
                                                    -> bool {
    if let Some(iter) = model.get_iter(path) {
        model.get_value(&iter, column).get().unwrap()
    } else {
        panic!("get_boolean_column_via_path: gtk_tree_model_get_iter failed");
    }
}

pub fn get_iter_to_last_row(model: &TreeModel) -> Option<TreeIter> {
    let n = model.iter_n_children(None);
    if n > 0 {
        model.iter_nth_child(None, n - 1)
    } else {
        None
    }
}

pub fn select_row(view: &TreeView, path: &TreePath, focus_column: &TreeViewColumn) {
    view.set_cursor(path, Some(focus_column), false);
    view.scroll_to_cell(Some(path), None as Option<&TreeViewColumn>, true, 0.5, 0.0);
    view.grab_focus();
}

pub fn select_row_by_guid(view: &TreeView, guid: &str, store_guid_column: i32,
                          focus_column: &TreeViewColumn) {
    let model = view.get_model().unwrap();
    // Begin the search at the last row. Usually the desired row is near the bottom.
    if let Some(iter) = get_iter_to_last_row(&model) {
        loop {
            let current_guid: String = model.get_value(&iter, store_guid_column).get().unwrap();
            if current_guid == guid {
                // Found it
                if let Some(path) = model.get_path(&iter) {
                    select_row(view, &path, &focus_column);
                    return;
                } else {
                    panic!("select_row_by_guid: after searching by guid and finding a match, \
                            get_path failed");
                }
            } else if !model.iter_previous(&iter) {
                // If we get here, we've looked at all the rows, without finding the one we're looking for.
                // Give up and select the last row
                select_last_row(view, &focus_column);
                return;
            }
        }
    }
}

pub fn column_index_to_column(view: &TreeView, column_index: i32) -> TreeViewColumn {
    if let Some(column) = view.get_column(column_index) {
        column
    } else {
        panic!("column_index_to_column: get_column returned None. Column index was {}",
               column_index);
    }
}

pub fn select_first_row(view: &TreeView, focus_column: &TreeViewColumn) {
    let model = view.get_model().unwrap();
    if let Some(iter) = model.get_iter_first() {
        let path = model.get_path(&iter).unwrap();
        select_row(view, &path, &focus_column);
    }
}

pub fn select_last_row(view: &TreeView, focus_column: &TreeViewColumn) {
    let model = view.get_model().unwrap();
    if let Some(iter) = get_iter_to_last_row(&model) {
        let path = model.get_path(&iter).unwrap();
        select_row(view, &path, &focus_column);
    }
}

pub fn update_string_column_via_path(store: &ListStore, path: &TreePath, new_string: &str,
                                     column: i32) {
    if let Some(iter) = store.get_iter(path) {
        // Write new_string to the store
        store.set(&iter, &[column as u32], &[&(new_string.to_string())]);
    } else {
        panic!("update_string_column_via_path: gtk_tree_model_get_iter_from_string failed");
    }
}

pub fn update_boolean_column_via_path(store: &ListStore, path: &TreePath, new_boolean: bool,
                                      column: i32) {
    if let Some(iter) = store.get_iter(path) {
        // Write new_boolean to the store
        store.set(&iter, &[column as u32], &[&new_boolean]);
    } else {
        panic!("update_boolean_column_via_path: gtk_tree_model_get_iter_from_string failed");
    }
}

pub fn create_tree_view_text_column(renderer: &CellRendererText, title: &str,
                                    store_column_index: i32)
                                    -> TreeViewColumn {
    let column = TreeViewColumn::new();
    column.pack_start(renderer, true);
    column.add_attribute(renderer, "text", store_column_index);
    column.set_title(title);
    column
}

pub fn create_tree_view_toggle_column(renderer: &CellRendererToggle, title: &str,
                                      store_column_index: i32)
                                      -> TreeViewColumn {
    let column = TreeViewColumn::new();
    column.pack_start(renderer, true);
    column.add_attribute(renderer, "active", store_column_index);
    column.set_title(title);
    column
}

pub fn create_and_enter_dialog_item<P: IsA<Widget>>(grid: &Grid, name: &str, row: i32, item: &P) {
    let label = Label::new(Some(name));
    label.set_halign(Align::End);
    grid.attach(&label, 1, row, 1, 1);
    grid.attach(item, 2, row, 1, 1);
}

pub fn find_search(find_command: &FindCommand, find_parameters: &RefCell<FindParameters>,
                   register_core: &RegisterCore, globals: &Globals)
                   -> Option<TreePath> {
    let find_parameters_borrow = find_parameters.borrow();
    if let Some(ref regex) = find_parameters_borrow.regex {
        let view = &register_core.view;
        let model = view.get_model().unwrap();
        let iter = match find_command {
            FindCommand::FindNextBackward => {
                // Try to get the path to the row last found
                if let Some(ref path) = find_parameters_borrow.path {
                    // Try to convert it into an iter
                    let iter = model.get_iter(&path).expect("Find: get_iter failed");
                    if !model.iter_previous(&iter) {
                        // Can't find previous, must be at beginning, wrap to the end
                        get_iter_to_last_row(&model).expect("find: get_iter_to_last_row failed")
                    } else {
                        iter
                    }
                } else {
                    // Must be at beginning, wrap to the end
                    get_iter_to_last_row(&model).expect("find: get_iter_to_last_row failed")
                }
            }
            FindCommand::FindNextForward => {
                // Try to get the path to the row last found
                if let Some(ref path) = find_parameters_borrow.path {
                    // Try to convert it into an iter
                    let iter = model.get_iter(&path).expect("Find: get_iter failed");
                    if !model.iter_next(&iter) {
                        // Can't find next, must be at end, wrap to the beginning
                        model.get_iter_first().expect("find: get_iter_first failed")
                    } else {
                        iter
                    }
                } else {
                    // Must be at end, wrap to the beginning
                    model.get_iter_first().expect("find: get_iter_first failed")
                }
            }
            FindCommand::FindBackward => {
                get_iter_to_last_row(&model).expect("find: get_iter_to_last_row failed")
            }
            FindCommand::FindForward => {
                model.get_iter_first().expect("find: get_iter_first failed")
            }
        };

        view.get_selection().unselect_all();
        let column_index = find_parameters_borrow.column_index
                                                 .expect("Find: non-existent column index");
        loop {
            let column_value: String =
                match find_parameters_borrow.column_type
                                            .expect("Find: in search, column_type is missing")
                {
                    Type::String => model.get_value(&iter, column_index).get().unwrap(),
                    Type::Bool => {
                        let temp: bool = model.get_value(&iter, column_index).get().unwrap();
                        format!("{}", temp)
                    }
                    _ => panic!("Find: invalid column type"),
                };
            if regex.is_match(&column_value) {
                // Found it
                let path = model.get_path(&iter);
                let column: TreeViewColumn =
                    view.get_column(find_parameters_borrow.default_view_column
                                                          .try_into()
                                                          .unwrap())
                        .expect("Find: in search, unable to convert column index to column");
                {
                    // Need inner scope to temporarily borrow path, which has to end before moving it in the return stmt
                    let temp_path = path.as_ref().unwrap();
                    view.scroll_to_cell(Some(temp_path), Some(&column), false, 0., 0.);
                    view.set_cursor(temp_path, Some(&column), false);
                }
                return path;
            }
            // Didn't find it
            match find_command {
                FindCommand::FindBackward | FindCommand::FindNextBackward => {
                    if !model.iter_previous(&iter) {
                        select_first_row(&view, &column_index_to_column(&view, column_index));
                        return None;
                    };
                }
                FindCommand::FindForward | FindCommand::FindNextForward => {
                    if !model.iter_next(&iter) {
                        select_last_row(&view, &column_index_to_column(&view, column_index));
                        return None;
                    };
                }
            }
        }
    } else {
        display_message_dialog("You must do a Find before a Find Next.", globals);
        None
    }
}

fn find_dialog(find_parameters: &RefCell<FindParameters>, register_core: &RegisterCore,
               globals: &Globals)
               -> bool {
    let regular_expression = Entry::new_with_buffer(&EntryBuffer::new(None));
    let column_choices = ComboBoxText::new();
    let grid = Grid::new();
    create_and_enter_dialog_item(&grid, "Find expression:", 1, &regular_expression);
    create_and_enter_dialog_item(&grid, "Column:", 2, &column_choices);
    let dialog = Dialog::new_with_buttons(Some("New Account"),
                                          Some(&register_core.window),
                                          DialogFlags::MODAL,
                                          &[("OK", ResponseType::Ok),
                                            ("Cancel", ResponseType::Cancel)]);
    let content_area = dialog.get_content_area();

    let mut find_parameters_borrow_mut = find_parameters.borrow_mut();

    // Populate the column_choices combo_box
    let column_names = find_parameters_borrow_mut.column_names;
    for column_name in column_names {
        column_choices.append_text(column_name);
    }
    // And set the active entry to the specified default column
    column_choices.set_active(Some(find_parameters_borrow_mut.default_view_column));

    content_area.add(&grid);
    dialog.show_all();
    if dialog.run() == ResponseType::Ok {
        // Build a case-insensitive regex
        if let Ok(regex) =
            Regex::new(format!("(?i){}",
                               regular_expression.get_text()
                                                 .expect("Find: failed to obtain regex from \
                                                          dialog")).as_str())
        {
            find_parameters_borrow_mut.regex = Some(regex);
            // Find the column she wants to search on
            let temp = column_choices.get_active_text()
                                     .expect("Find: failed to obtain column name from dialog");
            let active_text: &str = temp.as_str();
            for (i, column_name) in column_names.iter().enumerate() {
                if &active_text == column_name {
                    find_parameters_borrow_mut.column_index =
                        Some(find_parameters_borrow_mut.column_indices[i]);
                    find_parameters_borrow_mut.column_type =
                        Some(find_parameters_borrow_mut.column_types[i]);
                    dialog.destroy();
                    return true;
                }
            }
            panic!("Find: unable to locate specified column. This is a bug in Newcash. Please \
                    report to Don Allen.");
        } else {
            display_message_dialog("The find expression was not a valid regular expression.",
                                   globals);
            dialog.destroy();
            false
        }
    } else {
        dialog.destroy();
        false
    }
}

pub fn find(find_command: &FindCommand, find_parameters: &RefCell<FindParameters>,
            register_core: &RegisterCore, globals: &Globals) {
    let maybe_path = match find_command {
        FindCommand::FindForward | FindCommand::FindBackward => {
            if find_dialog(find_parameters, register_core, globals) {
                find_search(&find_command, find_parameters, register_core, globals)
            } else {
                None
            }
        }
        FindCommand::FindNextForward | FindCommand::FindNextBackward => {
            find_search(&find_command, find_parameters, register_core, globals)
        }
    };
    let mut temp = find_parameters.borrow_mut();
    temp.path = maybe_path;
}

pub fn get_selection_info(register_core: &RegisterCore, globals: &Globals)
                          -> Option<(TreeModel, TreeIter)> {
    let result = register_core.view.get_selection().get_selected();
    if result.is_none() {
        display_message_dialog("Improper selection", globals);
    }
    result
}
