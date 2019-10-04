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

use gdk::ModifierType;
use gtk::{
    ComboBoxText, Entry, ListStore, ScrolledWindow, TreePath, TreeStore, TreeView, Type, Window,
};
use regex::Regex;
use rusqlite::Connection;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// Constants
pub const ACCOUNT_TREE_STORE_GUID: i32 = 0;
pub const ACCOUNT_TREE_STORE_NAME: i32 = ACCOUNT_TREE_STORE_GUID + 1;
pub const ACCOUNT_TREE_STORE_FLAGS: i32 = ACCOUNT_TREE_STORE_NAME + 1;

pub const DATE_ERROR_MESSAGE: &str = "Invalid date input. Must be either YYYY-MM-DD,
a positve or negative number of days,
or one of =, +, -, _, t, m, or h.";
pub const DATE_SIZE: usize = 10; // Length, in characters, of an ISO-8601 date (YYYY-MM-DD)

// Types
#[derive(Debug)]
pub struct CommodityEditing {
    pub child_guid: Option<String>,
    pub commodity_item: ComboBoxText,
    pub pattern_item: Entry,
}

pub struct Globals {
    pub account_copy_buffer: RefCell<Option<String>>,
    pub account_registers: RefCell<HashMap<String, Rc<AccountRegister>>>,
    pub accounts_store: TreeStore,
    pub accounts_view: TreeView,
    pub accounts_window: Window,
    pub book_name: String,
    pub db: Connection,
    pub db_path: String,
    pub guid_processed: RefCell<HashSet<String>>,
    pub guid_to_full_path: RefCell<HashMap<String, String>>,
    pub modifiers: ModifierType,
    pub root_account_guid: Rc<String>,
    pub show_hidden: RefCell<bool>,
    pub transaction_registers: RefCell<HashMap<String, Rc<TransactionRegister>>>,
    pub unspecified_account_guid: String,
}

pub struct RegisterCore {
    pub view: TreeView,
    pub window: Window,
}

pub struct AccountRegister {
    pub core: RegisterCore,
    pub find_parameters: RefCell<FindParameters>,
    pub guid: String, // account guid
    pub scrolled_window: ScrolledWindow,
    pub shares_p: bool,
    pub store: ListStore,
}

pub struct TransactionRegister {
    pub account_register: Rc<AccountRegister>,
    pub core: RegisterCore,
    pub description: String,
    pub guid: String, // transaction guid
    pub store: ListStore,
}

pub struct CommoditiesRegister {
    pub core: RegisterCore,
    pub find_parameters: RefCell<FindParameters>,
    pub scrolled_window: ScrolledWindow,
    pub store: ListStore,
}

pub struct CommodityRegister {
    pub core: RegisterCore,
    pub guid: String,
    pub scrolled_window: ScrolledWindow,
    pub store: ListStore,
}

pub struct StockSplitsRegister {
    pub commodity_guid: Rc<String>,
    pub core: RegisterCore,
    pub fullname: String,
    pub scrolled_window: ScrolledWindow,
    pub store: ListStore,
}

pub struct FindParameters {
    // The column index from previous find, if any
    pub column_index: Option<i32>,
    // Path of previous find, if any
    pub path: Option<TreePath>,
    // The regex that was used in previous find, if any
    pub regex: Option<Regex>,
    // Type of previously found cell, if any
    pub column_type: Option<Type>,
    // Names and types of columns of register we are searching
    pub column_names: &'static [&'static str],
    pub column_indices: &'static [i32],
    pub column_types: &'static [Type],
    pub default_store_column: i32,
    pub default_view_column: u32,
}

// Commands for find
pub enum FindCommand {
    FindBackward,
    FindForward,
    FindNextBackward,
    FindNextForward,
}

// Reasons for calling refresh_transaction_registers
pub enum WhatChanged {
    AccountNameChanged,
    TransactionChanged,
    SplitEdited,
}
