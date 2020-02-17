extern crate rusqlite;
#[macro_use]
extern crate rust_library;
extern crate libc;

use rusqlite::{params, Connection, LoadExtensionGuard, Statement};
use rust_library::constants::{
    ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS, ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES,
    ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME, ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES,
    ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE, ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK,
    ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED, EPSILON,
};
use std::cmp::Ordering;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

mod constants;
mod queries;

// Types
struct Account {
    name: String,
    guid: String,
    value: f64,
    flags: i32,
    children: Vec<Account>,
}

struct AccountStatements<'l> {
    marketable_asset_value: Statement<'l>,
    non_marketable_asset_and_liability_value: Statement<'l>,
    income_and_expenses_value: Statement<'l>,
    account_children: Statement<'l>,
}

impl Account {
    // This routine takes the an account as self and adds the entire tree of descendents,
    // including the cumulative value of those descendents in its value slot.
    fn build_account_tree(
        &mut self, statements: &mut AccountStatements, julian_begin_date_time: f64,
        julian_end_date_time: f64,
    ) {
        {
            // Create child accounts and get their data from the database
            let children_iter = statements
                .account_children
                .query_map(params![self.guid], |row| {
                    Ok(Account {
                        name: row.get(0).unwrap(),
                        guid: row.get(1).unwrap(),
                        value: 0.0,
                        flags: row.get(2).unwrap(),
                        children: Vec::new(),
                    })
                })
                .unwrap();
            for wrapped_child in children_iter {
                let mut child = wrapped_child.unwrap();
                // The asset and liability statements take two arguments, so set that up here
                // and change if it turns out to be an income-expense statement, which requires
                // a third argument
                child.value = if (self.flags & ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS) != 0 {
                    if (self.flags & ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE) != 0 {
                        statements
                            .marketable_asset_value
                            .query_row(params![child.guid, julian_end_date_time], get_result!(f64))
                            .unwrap()
                    } else {
                        statements
                            .non_marketable_asset_and_liability_value
                            .query_row(params![child.guid, julian_end_date_time], get_result!(f64))
                            .unwrap()
                    }
                } else if (self.flags & ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES) != 0 {
                    statements
                        .non_marketable_asset_and_liability_value
                        .query_row(params![child.guid, julian_end_date_time], get_result!(f64))
                        .unwrap()
                } else if (self.flags
                    & (ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME | ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES))
                    != 0
                {
                    statements
                        .income_and_expenses_value
                        .query_row(
                            params![child.guid, julian_end_date_time, julian_begin_date_time],
                            get_result!(f64),
                        )
                        .unwrap()
                } else {
                    0.0
                };
                child.flags |= self.flags
                    & (ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE
                        | ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS
                        | ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED
                        | ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES
                        | ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME
                        | ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES
                        | ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK);
                self.children.push(child);
            }
        }
        if !self.children.is_empty() {
            for child in &mut self.children {
                child.build_account_tree(statements, julian_begin_date_time, julian_end_date_time);
                self.value += child.value;
            }
            // Sort the children
            if (self.flags
                & (ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS | ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES))
                != 0
            {
                // Descending
                self.children.sort_unstable_by(|a, b| {
                    if b.value < a.value {
                        Ordering::Less
                    } else if b.value > a.value {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                });
            } else {
                // Ascending
                self.children.sort_unstable_by(|a, b| {
                    if a.value < b.value {
                        Ordering::Less
                    } else if a.value > b.value {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                });
            }
        }
    }
}

// Procedures
fn escapify(s: &str) -> String {
    s.replace("%", "\\%").replace("_", "\\_").replace("&", "\\&").replace("$", "\\$")
}

fn replicate_string(s: &str, n: u8) -> String {
    let mut result: String = String::with_capacity((n as usize) * s.len());
    for _ in 0..n {
        result.push_str(s);
    }
    result
}

fn display_account(account: &Account, depth: u8, max_depth: u8, italic: bool) -> String {
    if italic {
        format!(
            "{}\\textit{{\\small {}}}{}\\textit{{\\small {:8.0}}}{}\\\\\n",
            replicate_string("\\ ", depth * 4),
            escapify(&(account.name)),
            replicate_string("&", max_depth - depth),
            account.value,
            replicate_string("&", depth)
        )
    } else {
        format!(
            "{} \\small {}{} \\small {:8.0}{}\\\\\n",
            replicate_string("\\ ", depth * 4),
            escapify(&(account.name)),
            replicate_string("&", max_depth - depth),
            account.value,
            replicate_string("&", depth)
        )
    }
}

fn write_report_subsection(
    account: &Account, depth: u8, max_depth: u8, writer: &mut BufWriter<File>,
) {
    if (depth < max_depth) && (account.value.abs() > EPSILON) {
        writer
            .write_all(
                display_account(
                    account,
                    depth,
                    max_depth,
                    (account.flags & ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED) != 0,
                )
                .as_bytes(),
            )
            .unwrap();
        for child in account.children.iter() {
            write_report_subsection(child, depth + 1, max_depth, writer);
        }
    }
}

fn main() {
    const BEGIN_DATE: usize = 1;
    const END_DATE: usize = BEGIN_DATE + 1;
    const DEPTH: usize = END_DATE + 1;
    const DB_FILE_INDEX: usize = DEPTH + 1;
    const REPORT_FILE_INDEX: usize = DB_FILE_INDEX + 1;
    const EXTENSIONS_LIBRARY_FILE_INDEX: usize = REPORT_FILE_INDEX + 1;
    const N_ARGS: usize = EXTENSIONS_LIBRARY_FILE_INDEX + 1;

    // Check that the number of arguments is correct
    if env::args().count() != N_ARGS {
        panic!(
            "Incorrect number of command line arguments: {}. Should be {}.
Usage: newcashReportGenerator beginDate endDate depth pathToDatabase pathToTexFile
    pathToTSVFile pathToSqliteExtensionsLibrary",
            std::env::args().count(),
            N_ARGS
        );
    }

    // Get args
    let begin_date = env::args().nth(BEGIN_DATE).unwrap();
    let begin_date_time = format!("{} 00:00:00", begin_date);
    let end_date = env::args().nth(END_DATE).unwrap();
    let end_date_time = format!("{} 23:59:59", end_date);
    let max_depth: u8 = env::args().nth(DEPTH).unwrap().parse().unwrap();

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    // Open the output file for the reports
    let mut report_file_writer =
        BufWriter::new(File::create(env::args().nth(REPORT_FILE_INDEX).unwrap()).unwrap());

    // Load sqlite extensions, so we have math functions
    let temp = env::args().nth(EXTENSIONS_LIBRARY_FILE_INDEX).unwrap();
    let extensions_file_path = Path::new(&temp);
    {
        let _guard = LoadExtensionGuard::new(&db).unwrap();
        db.load_extension(extensions_file_path, None).unwrap();
    }

    // Convert beginDateTime and endDateTime to julian. We will need it later for roi calculations
    let julian_begin_date_time: f64;
    let julian_end_date_time: f64;
    {
        let mut julian_conversion_statement = db.prepare(queries::JULIAN_CONVERSION_SQL).unwrap();
        julian_end_date_time = julian_conversion_statement
            .query_row(params![end_date_time], get_result!(f64))
            .unwrap();
        julian_begin_date_time = julian_conversion_statement
            .query_row(params![begin_date_time], get_result!(f64))
            .unwrap();
    }

    // Get root account data
    let mut root: Account = db
        .query_row(queries::ROOT_DATA_SQL, params![], |row| {
            Ok(Account {
                name: row.get(0).unwrap(),
                guid: row.get(1).unwrap(),
                flags: row.get(2).unwrap(),
                value: 0.0,
                children: Vec::new(),
            })
        })
        .unwrap();

    // Prepare to build the account tree
    let mut account_statements: AccountStatements = AccountStatements {
        marketable_asset_value: db.prepare(queries::MARKETABLE_ASSET_VALUE_SQL).unwrap(),
        non_marketable_asset_and_liability_value: db
            .prepare(queries::NON_MARKETABLE_ASSET_AND_LIABILITY_VALUE_SQL)
            .unwrap(),
        income_and_expenses_value: db.prepare(queries::INCOME_AND_EXPENSES_VALUE_SQL).unwrap(),
        account_children: db.prepare(queries::ACCOUNT_CHILDREN_SQL).unwrap(),
    };
    root.build_account_tree(&mut account_statements, julian_begin_date_time, julian_end_date_time);

    // Write the document header
    report_file_writer.write_all(constants::DOCUMENT_HEADER.as_bytes()).unwrap();

    // Balance sheet
    // Write the balance sheet header
    report_file_writer
        .write_all(
            format!(
                "\\newpage
\\section{{Balance Sheet}}
\\begin{{longtable}} {{|l{0}|}}
\\hline
\\endhead
\\hline
\\endfoot
",
                "|r".repeat(max_depth as usize)
            )
            .as_bytes(),
        )
        .unwrap();

    fn find_sub_tree(parent: &Account, type_bit: i32) -> &Account {
        for child in parent.children.iter() {
            if child.flags & type_bit != 0 {
                return &child;
            }
        }
        panic!("find_sub_tree failed to find a sub-tree having the flag bit {}", type_bit);
    }

    // Assets
    let assets_account_tree = find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS);
    write_report_subsection(assets_account_tree, 0, max_depth, &mut report_file_writer);
    report_file_writer.write_all(constants::ASSETS_FOOTER.as_bytes()).unwrap();

    // Liabilities
    let liabilities_account_tree = find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES);
    write_report_subsection(liabilities_account_tree, 0, max_depth, &mut report_file_writer);
    report_file_writer.write_all(constants::LIABILITIES_FOOTER.as_bytes()).unwrap();

    // Income statement
    // Write the income statement header
    report_file_writer
        .write_all(
            format!(
                "
\\newpage
\\section{{Income Statement ({0} through {1})}}
\\begin{{longtable}} {{|l{2}|}}
\\hline
\\endhead
\\hline
\\endfoot
",
                begin_date,
                end_date,
                "|r".repeat(max_depth as usize)
            )
            .as_bytes(),
        )
        .unwrap();

    // Income
    let income_account_tree = find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME);
    write_report_subsection(income_account_tree, 0, max_depth, &mut report_file_writer);
    report_file_writer.write_all(constants::INCOME_FOOTER.as_bytes()).unwrap();

    // Expenses
    let expenses_account_tree = find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES);
    write_report_subsection(expenses_account_tree, 0, max_depth, &mut report_file_writer);
    report_file_writer.write_all(constants::EXPENSES_FOOTER.as_bytes()).unwrap();

    // Net worth
    report_file_writer
        .write_all(
            format!(
                "\\newpage
\\section{{Net Worth}}
Current net worth is \\${:.0}
\\section{{Net Cash Flow Into Assets/Liabilities from {} to {}}}
Net cash flow: \\${:.0}
\\newpage
",
                assets_account_tree.value + liabilities_account_tree.value,
                begin_date,
                end_date,
                -(income_account_tree.value + expenses_account_tree.value),
            )
            .as_bytes(),
        )
        .unwrap();
}
