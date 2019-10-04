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

use constants::Globals;
use gtk::{
    Calendar, CalendarExt, ContainerExt, Dialog, DialogExt, DialogFlags, ResponseType, WidgetExt,
    Window,
};
use utilities::maybe_date;

pub fn display_calendar(initial_date: &str, parent_window: &Window, globals:&Globals) -> Option<String> {
    let calendar = Calendar::new();
    let dialog = Dialog::new_with_buttons(Some("Select date"),
                                          Some(parent_window),
                                          DialogFlags::MODAL,
                                          &[("OK", ResponseType::Ok),
                                            ("Cancel", ResponseType::Cancel)]);
    let content_area = dialog.get_content_area();
    content_area.add(&calendar);

    // Set calendar to the date currently in the selected row, if there is one
    // and it is in ISO-8601 format
    if !initial_date.is_empty() && maybe_date(initial_date, globals) {
        // Split the date into its components as strings
        let split_date: Vec<&str> = initial_date.split('-').collect();
        // And convert to u32s
        let year: u32 = split_date[0].parse().unwrap();
        let month: u32 = split_date[1].parse().unwrap();
        let day: u32 = split_date[2][0..2].parse().unwrap(); // Need the slice because this might be a timestamp
        calendar.select_month(month - 1, year);
        calendar.select_day(day);
        calendar.mark_day(day);
    }

    // Set default response to 'ok'
    dialog.set_default_response(ResponseType::Ok);

    // And handle the day-selected signal to put a marker on the new date
    calendar.connect_day_selected(|calendar| {
                let (_, _, day) = calendar.get_date();
                calendar.clear_marks();
                calendar.mark_day(day);
            });

    dialog.show_all();
    if dialog.run() == ResponseType::Ok {
        // Retrieve the new data from the calendar
        let (year, month, day) = calendar.get_date();
        // Destroy the dialog
        dialog.destroy();
        // Format and return the date
        Some(format!("{:4}-{:02}-{:02}", year, month + 1, day))
    } else {
        dialog.destroy();
        None
    }
}
