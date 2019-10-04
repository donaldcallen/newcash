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
use std::sync::mpsc;
use std::thread;

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

struct OpenPosition {
    header: OpenPositionHeader,
    current_value: Option<f64>,
    capital_gain: Option<f64>,
    total_gain: Option<f64>,
    annualized_return: Option<f64>,
    total_annualized_return: Option<f64>,
    most_recent_quote_timestamp: Option<f64>,
}

struct OpenPositionHeader {
    symbol: String,
    name: String,
    commodity_guid: String,
    quantity: f64,
}

enum InvestmentReportType {
    Value,
    CapitalGain,
    TotalCapitalGain,
    AnnualizedReturn,
    TotalAnnualizedReturn,
    MostRecentQuote,
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
    fn build_account_tree(&mut self, statements: &mut AccountStatements,
                          julian_begin_date_time: f64, julian_end_date_time: f64) {
        {
            // Create child accounts and get their data from the database
            let children_iter = statements.account_children
                                          .query_map(params![self.guid], |row| {
                                              Ok(Account { name: row.get(0).unwrap(),
                                                           guid: row.get(1).unwrap(),
                                                           value: 0.0,
                                                           flags: row.get(2).unwrap(),
                                                           children: Vec::new() })
                                          })
                                          .unwrap();
            for wrapped_child in children_iter {
                let mut child = wrapped_child.unwrap();
                // The asset and liability statements take two arguments, so set that up here
                // and change if it turns out to be an income-expense statement, which requires
                // a third argument
                child.value = if (self.flags & ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS) != 0 {
                    if (self.flags & ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE) != 0 {
                        statements.marketable_asset_value
                                  .query_row(params![child.guid, julian_end_date_time],
                                             get_result!(f64))
                                  .unwrap()
                    } else {
                        statements.non_marketable_asset_and_liability_value
                                  .query_row(params![child.guid, julian_end_date_time],
                                             get_result!(f64))
                                  .unwrap()
                    }
                } else if (self.flags & ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES) != 0 {
                    statements.non_marketable_asset_and_liability_value
                              .query_row(params![child.guid, julian_end_date_time],
                                         get_result!(f64))
                              .unwrap()
                } else if (self.flags
                           & (ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME
                              | ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES))
                          != 0
                {
                    statements.income_and_expenses_value
                              .query_row(params![child.guid,
                                                 julian_end_date_time,
                                                 julian_begin_date_time],
                                         get_result!(f64))
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
                child.build_account_tree(statements,
                                         julian_begin_date_time,
                                         julian_end_date_time);
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
    s.replace("%", "\\%")
     .replace("_", "\\_")
     .replace("&", "\\&")
     .replace("$", "\\$")
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
        format!("{}\\textit{{\\small {}}}{}\\textit{{\\small {:8.0}}}{}\\\\\n",
                replicate_string("\\ ", depth * 4),
                escapify(&(account.name)),
                replicate_string("&", max_depth - depth),
                account.value,
                replicate_string("&", depth))
    } else {
        format!("{} \\small {}{} \\small {:8.0}{}\\\\\n",
                replicate_string("\\ ", depth * 4),
                escapify(&(account.name)),
                replicate_string("&", max_depth - depth),
                account.value,
                replicate_string("&", depth))
    }
}

fn write_report_subsection(account: &Account, depth: u8, max_depth: u8,
                           writer: &mut BufWriter<File>) {
    if (depth < max_depth) && (account.value.abs() > EPSILON) {
        writer.write_all(display_account(account,
                                         depth,
                                         max_depth,
                                         (account.flags
                                          & ACCOUNT_FLAG_SELF_AND_DESCENDENTS_ARE_TAX_RELATED)
                                         != 0).as_bytes())
              .unwrap();
        for child in account.children.iter() {
            write_report_subsection(child, depth + 1, max_depth, writer);
        }
    }
}

fn investment_report(open_positions: &mut Vec<OpenPosition>, investment_report_tex: &mut String,
                     report_type: &InvestmentReportType,
                     date_conversion_statement: &mut Statement) {
    fn get_position_current_value(open_position: &OpenPosition) -> Option<f64> {
        open_position.current_value
    }
    fn get_position_current_most_recent_quote(open_position: &OpenPosition) -> Option<f64> {
        open_position.most_recent_quote_timestamp
    }
    fn get_position_capital_gain(open_position: &OpenPosition) -> Option<f64> {
        open_position.capital_gain
    }
    fn get_position_total_gain(open_position: &OpenPosition) -> Option<f64> {
        open_position.total_gain
    }
    fn get_position_annualized_return(open_position: &OpenPosition) -> Option<f64> {
        open_position.annualized_return
    }
    fn get_position_total_annualized_return(open_position: &OpenPosition) -> Option<f64> {
        open_position.total_annualized_return
    }

    // Sort the open positions based on the report we are working on
    let accessor = match report_type {
        InvestmentReportType::Value => get_position_current_value,
        InvestmentReportType::MostRecentQuote => get_position_current_most_recent_quote,
        InvestmentReportType::CapitalGain => get_position_capital_gain,
        InvestmentReportType::TotalCapitalGain => get_position_total_gain,
        InvestmentReportType::AnnualizedReturn => get_position_annualized_return,
        InvestmentReportType::TotalAnnualizedReturn => get_position_total_annualized_return,
    };
    // Quotes are sorted in ascending order. Everything else in descending order.
    // The idea is have the most important information, the items most likely to need
    // attention, at the top.
    match report_type {
        InvestmentReportType::MostRecentQuote => {
            open_positions.sort_unstable_by(|p1, p2| match accessor(p1) {
                              None => Ordering::Less,
                              Some(value1) => match accessor(p2) {
                                  None => Ordering::Greater,
                                  Some(value2) => {
                                      if value1 > value2 {
                                          Ordering::Greater
                                      } else if value1 < value2 {
                                          Ordering::Less
                                      } else {
                                          Ordering::Equal
                                      }
                                  }
                              },
                          })
        }
        _ => open_positions.sort_unstable_by(|p1, p2| match accessor(p1) {
                               None => Ordering::Greater,
                               Some(value1) => match accessor(p2) {
                                   None => Ordering::Less,
                                   Some(value2) => {
                                       if value1 < value2 {
                                           Ordering::Greater
                                       } else if value1 > value2 {
                                           Ordering::Less
                                       } else {
                                           Ordering::Equal
                                       }
                                   }
                               },
                           }),
    };
    for open_position in open_positions.iter() {
        match accessor(open_position) {
            None => {
                match report_type {
                    InvestmentReportType::Value => investment_report_tex.push_str(
                        format!(
                            "{} & {:8.0} & --------\\\\\n",
                            escapify(&open_position.header.name),
                            open_position.header.quantity
                        )
                        .as_str(),
                    ),
                    InvestmentReportType::MostRecentQuote => investment_report_tex.push_str(
                        format!("{} & --------\\\\\n", escapify(&open_position.header.name),)
                            .as_str(),
                    ),
                    InvestmentReportType::CapitalGain | InvestmentReportType::TotalCapitalGain => {
                        investment_report_tex.push_str(
                            format!("{} & --------\\\\\n", escapify(&open_position.header.name))
                                .as_str(),
                        )
                    }
                    InvestmentReportType::AnnualizedReturn
                    | InvestmentReportType::TotalAnnualizedReturn => investment_report_tex
                        .push_str(
                            format!(
                                "{} & -------\\%\\\\\n",
                                escapify(&open_position.header.name)
                            )
                            .as_str(),
                        ),
                };
            }
            Some(value) => {
                match report_type {
                    InvestmentReportType::Value => {
                        investment_report_tex.push_str(format!("{} & {:8.0} & {:8.0}\\\\\n",
                                                               escapify(&open_position.header
                                                                                      .name),
                                                               open_position.header.quantity,
                                                               value).as_str())
                    }
                    InvestmentReportType::MostRecentQuote => {
                        let timestamp = if open_position.most_recent_quote_timestamp.is_some() {
                            date_conversion_statement
                                .query_row(
                                    &[open_position.most_recent_quote_timestamp.as_ref().unwrap()],
                                    get_result!(string),
                                )
                                .unwrap()
                        } else {
                            "None".to_string()
                        };
                        investment_report_tex.push_str(format!("{} & {}\\\\\n",
                                                               escapify(&open_position.header
                                                                                      .name),
                                                               timestamp).as_str())
                    }
                    InvestmentReportType::CapitalGain | InvestmentReportType::TotalCapitalGain => {
                        investment_report_tex.push_str(format!("{} & {:8.0}\\\\\n",
                                                               escapify(&open_position.header
                                                                                      .name),
                                                               value).as_str())
                    }
                    InvestmentReportType::AnnualizedReturn
                    | InvestmentReportType::TotalAnnualizedReturn => {
                        investment_report_tex.push_str(format!("{} & {:7.1}\\%\\\\\n",
                                                               escapify(&open_position.header
                                                                                      .name),
                                                               value).as_str())
                    }
                };
            }
        }
    }
}

fn main() {
    const BEGIN_DATE: usize = 1;
    const END_DATE: usize = BEGIN_DATE + 1;
    const DEPTH: usize = END_DATE + 1;
    const DB_FILE_INDEX: usize = DEPTH + 1;
    const TEX_FILE_INDEX: usize = DB_FILE_INDEX + 1;
    const TSV_FILE_INDEX: usize = TEX_FILE_INDEX + 1;
    const EXTENSIONS_LIBRARY_FILE_INDEX: usize = TSV_FILE_INDEX + 1;
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
    let tsv_file_path = env::args().nth(TSV_FILE_INDEX).unwrap();

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    // Open the output file for the reports
    let mut tex_file_writer =
        BufWriter::new(File::create(env::args().nth(TEX_FILE_INDEX).unwrap()).unwrap());

    // Load sqlite extensions, so we have math functions
    let unix_extensions_file_path = env::args().nth(EXTENSIONS_LIBRARY_FILE_INDEX).unwrap();
    let extensions_file_path = Path::new(&unix_extensions_file_path);
    {
        let _guard = LoadExtensionGuard::new(&db).unwrap();
        db.load_extension(extensions_file_path, None).unwrap();
    }

    // Convert beginDateTime and endDateTime to julian. We will need it later for roi calculations
    let julian_begin_date_time: f64;
    let julian_end_date_time: f64;
    {
        let mut julian_conversion_statement = db.prepare(queries::JULIAN_CONVERSION_SQL).unwrap();
        julian_end_date_time = julian_conversion_statement.query_row(params![end_date_time],
                                                                     get_result!(f64))
                                                          .unwrap();
        julian_begin_date_time = julian_conversion_statement.query_row(params![begin_date_time],
                                                                       get_result!(f64))
                                                            .unwrap();
    }

    // In its own thread, go through each commodity for which there is an open position
    // Channel to receive the result
    let (tx_result, rx_result) = mpsc::channel();
    {
        // Channel to send julian_end_date
        let (tx_date, rx_date) = mpsc::channel();
        thread::spawn(move || {
            let mut investment_report_tex =
                String::with_capacity(constants::INVESTMENT_REPORT_SIZE);
            // Open the database within the thread
            let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();
            // Load sqlite extensions, so we have math functions
            let unix_extensions_file_path = env::args().nth(EXTENSIONS_LIBRARY_FILE_INDEX).unwrap();
            let extensions_file_path = Path::new(&unix_extensions_file_path);
            {
                let _guard = LoadExtensionGuard::new(&db).unwrap();
                db.load_extension(extensions_file_path, None).unwrap();
            }
            let mut open_positions: Vec<OpenPosition> = {
                let julian_end_date_time: f64 = rx_date.recv().unwrap();
                // Annualized return on investment. Note that the time interval is in days and the dates are julian.
                let roi = |begin_value: f64, end_value: f64, then: f64| -> Option<f64> {
                    if begin_value == 0.0 {
                        None
                    } else {
                        Some(((end_value / begin_value).powf(365.0
                                                             / (julian_end_date_time - then))
                              - 1.0)
                             * 100.0)
                    }
                };

                let mut open_positions: Vec<OpenPosition> = Vec::new();

                let mut open_positions_statement = db.prepare(queries::OPEN_POSITIONS_SQL).unwrap();
                let mut most_recent_zero_crossing_statement =
                    db.prepare(queries::MOST_RECENT_ZERO_CROSSING_SQL).unwrap();
                let mut get_position_basis_statement =
                    db.prepare(queries::GET_POSITION_BASIS_SQL).unwrap();
                let mut price_statement = db.prepare(queries::PRICE_SQL).unwrap();
                let mut dividend_statement = db.prepare(queries::DIVIDEND_SQL).unwrap();

                let open_positions_iter =
                    open_positions_statement.query_map(params![julian_end_date_time], |row| {
                                                Ok(OpenPositionHeader { commodity_guid:
                                                                            row.get(0).unwrap(),
                                                                        symbol: row.get(1)
                                                                                   .unwrap(),
                                                                        name: row.get(2)
                                                                                 .unwrap(),
                                                                        quantity: row.get(3)
                                                                                     .unwrap() })
                                            })
                                            .unwrap();
                for wrapped_open_position_header in open_positions_iter {
                    let open_position_header = wrapped_open_position_header.unwrap();
                    // Find the most recent zero crossing
                    let most_recent_zero_crossing: f64 = {
                        let mut remainder: f64 = open_position_header.quantity;
                        let mut post_date = None;
                        let most_recent_zero_crossing_iter = most_recent_zero_crossing_statement
                            .query_map(
                                params![open_position_header.commodity_guid, julian_end_date_time],
                                get_result!(f64_f64),
                                )
                            .unwrap();
                        for wrapped_possible_zero_crossing in most_recent_zero_crossing_iter {
                            let (possible_post_date, quantity) =
                                wrapped_possible_zero_crossing.unwrap();
                            remainder -= quantity;
                            if remainder.abs() < 0.1 {
                                post_date = Some(possible_post_date);
                                break;
                            }
                        }
                        match post_date {
                            None => panic!("Unable to find most recent zero crossing for {}",
                                           open_position_header.name),
                            Some(temp) => temp,
                        }
                    };
                    // Now compute the position basis
                    let position_basis: f64 = {
                        let mut basis_balance = 0.;
                        let mut quantity_balance = 0.;
                        let get_position_basis_iter =
                            get_position_basis_statement.query_map(params![open_position_header.commodity_guid,
                                                                   most_recent_zero_crossing,
                                                                   julian_end_date_time],
                                                                   get_result!(f64_f64))
                            .unwrap();
                        for wrapped_basis_data in get_position_basis_iter {
                            let (quantity, value) = wrapped_basis_data.unwrap();
                            // If the quantity has the same sign as the current share balance,
                            // then this is an opening transaction
                            if quantity.signum() == open_position_header.quantity.signum() {
                                basis_balance += value;
                            } else {
                                // Closing transaction. The value of this transaction is
                                // the quantity * the average basis price thus far,
                                // which is basis_balance/quantity_balance.
                                // The value is not the value in the split.
                                // It is the amount that we would have used from
                                // the basis to compute the capital gain of this closing transaction.
                                // This amount is deducted from the running
                                // basis_balance by virtue of the
                                // quantity having the opposite sign of the opening transaction.
                                basis_balance =
                                    basis_balance + quantity * basis_balance / quantity_balance;
                            }
                            quantity_balance += quantity;
                        }
                        basis_balance
                    };
                    // Obtain values that are functions of price
                    match price_statement.query_row(params![open_position_header.commodity_guid,
                                                            julian_end_date_time],
                                                    get_result!(f64_f64))
                    {
                        Ok((price, most_recent_quote_timestamp)) => {
                            let dividends =
                                match dividend_statement.query_row(params![open_position_header.commodity_guid,
                                                                   most_recent_zero_crossing,
                                                                   julian_end_date_time],
                                                                   get_result!(f64))
                                {
                                    Err(_) => 0.0,
                                    Ok(dividends) => dividends,
                                };
                            let current_value: f64 = open_position_header.quantity * price;
                            open_positions.push(OpenPosition { header: open_position_header,
                                                 current_value: Some(current_value),
                                                 capital_gain: Some(current_value
                                                                    - position_basis),
                                                 total_gain: Some(current_value
                                                                  - position_basis
                                                                  + dividends),
                                                 annualized_return:
                                                     roi(position_basis,
                                                         current_value,
                                                         most_recent_zero_crossing),
                                                 total_annualized_return:
                                                     roi(position_basis,
                                                         current_value + dividends,
                                                         most_recent_zero_crossing),
                                                 most_recent_quote_timestamp:
                                                     Some(most_recent_quote_timestamp) });
                        }
                        Err(_) => {
                            open_positions.push(OpenPosition { header: open_position_header,
                                                               current_value: None,
                                                               capital_gain: None,
                                                               total_gain: None,
                                                               annualized_return: None,
                                                               total_annualized_return: None,
                                                               most_recent_quote_timestamp:
                                                                   None });
                        }
                    }
                }
                open_positions
            };

            // Investments
            let mut date_conversion_statement = db.prepare(queries::CONVERT_JULIAN_DAY_SQL).unwrap();
            investment_report_tex.push_str(constants::INVESTMENTS_HEADER);

            // Open positions subsection header
            investment_report_tex.push_str(constants::OPEN_POSITIONS_SUBSECTION_HEADER);
            investment_report(&mut open_positions,
                              &mut investment_report_tex,
                              &InvestmentReportType::Value,
                              &mut date_conversion_statement);
            investment_report_tex.push_str(constants::OPEN_POSITIONS_SUBSECTION_FOOTER);

            investment_report_tex.push_str(constants::OPEN_POSITIONS_QUOTES_SUBSECTION_HEADER);
            investment_report(&mut open_positions,
                              &mut investment_report_tex,
                              &InvestmentReportType::MostRecentQuote,
                              &mut date_conversion_statement);
            investment_report_tex.push_str(constants::OPEN_POSITIONS_SUBSECTION_FOOTER);

            investment_report_tex.push_str(constants::CAPITAL_GAIN_SUBSECTION_HEADER);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_HEADER);
            investment_report(&mut open_positions,
                              &mut investment_report_tex,
                              &InvestmentReportType::CapitalGain,
                              &mut date_conversion_statement);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_FOOTER);

            investment_report_tex.push_str(constants::TOTAL_CAPITAL_GAIN_SUBSECTION_HEADER);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_HEADER);
            investment_report(&mut open_positions,
                              &mut investment_report_tex,
                              &InvestmentReportType::TotalCapitalGain,
                              &mut date_conversion_statement);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_FOOTER);

            investment_report_tex.push_str(constants::ANNUALIZED_GAIN_SUBSECTION_HEADER);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_HEADER);
            investment_report(&mut open_positions,
                              &mut investment_report_tex,
                              &InvestmentReportType::AnnualizedReturn,
                              &mut date_conversion_statement);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_FOOTER);

            investment_report_tex.push_str(constants::TOTAL_ANNUALIZED_GAIN_SUBSECTION_HEADER);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_HEADER);
            investment_report(&mut open_positions,
                              &mut investment_report_tex,
                              &InvestmentReportType::TotalAnnualizedReturn,
                              &mut date_conversion_statement);
            investment_report_tex.push_str(constants::INVESTMENT_SUBSECTION_FOOTER);

            investment_report_tex.push_str(constants::DOCUMENT_FOOTER);

            // Possibly open and write the .tsv file of open positions,
            // for use as a Google spreadsheet with which I can
            // track recent performance
            if tsv_file_path != "Nothing" {
                let mut tsv_file_writer = BufWriter::new(File::create(tsv_file_path).unwrap());
                for open_position in open_positions.iter() {
                    tsv_file_writer.write_all(format!("{}\t{}\t{}\n",
                                                      open_position.header.symbol,
                                                      open_position.header.name,
                                                      open_position.header.quantity).as_bytes())
                                   .unwrap();
                }
            }
            tx_result.send(investment_report_tex).unwrap();
        });
        tx_date.send(julian_end_date_time).unwrap();
    }

    // Get root account data
    let mut root: Account = db.query_row(queries::ROOT_DATA_SQL, params![], |row| {
                                  Ok(Account { name: row.get(0).unwrap(),
                                               guid: row.get(1).unwrap(),
                                               flags: row.get(2).unwrap(),
                                               value: 0.0,
                                               children: Vec::new() })
                              })
                              .unwrap();

    // Prepare to build the account tree
    let mut account_statements: AccountStatements =
        AccountStatements { marketable_asset_value:
                                db.prepare(queries::MARKETABLE_ASSET_VALUE_SQL).unwrap(),
                            non_marketable_asset_and_liability_value:
                                db.prepare(queries::NON_MARKETABLE_ASSET_AND_LIABILITY_VALUE_SQL)
                                  .unwrap(),
                            income_and_expenses_value:
                                db.prepare(queries::INCOME_AND_EXPENSES_VALUE_SQL).unwrap(),
                            account_children: db.prepare(queries::ACCOUNT_CHILDREN_SQL).unwrap()};
    root.build_account_tree(&mut account_statements,
                            julian_begin_date_time,
                            julian_end_date_time);

    // Write the document header
    tex_file_writer.write_all(constants::DOCUMENT_HEADER.as_bytes())
                   .unwrap();

    // Balance sheet
    // Write the balance sheet header
    tex_file_writer.write_all(format!(
        "\\newpage
\\section{{Balance Sheet}}
\\begin{{longtable}} {{|l{0}|}}
\\hline
\\endhead
\\hline
\\endfoot
",
        "|r".repeat(max_depth as usize)
    ).as_bytes())
                   .unwrap();

    fn find_sub_tree(parent: &Account, type_bit: i32) -> &Account {
        for child in parent.children.iter() {
            if child.flags & type_bit != 0 {
                return &child;
            }
        }
        panic!("find_sub_tree failed to find a sub-tree having the flag bit {}",
               type_bit);
    }

    // Assets
    let assets_account_tree = find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS);
    write_report_subsection(assets_account_tree, 0, max_depth, &mut tex_file_writer);
    tex_file_writer.write_all(constants::ASSETS_FOOTER.as_bytes())
                   .unwrap();

    // Liabilities
    let liabilities_account_tree = find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES);
    write_report_subsection(liabilities_account_tree, 0, max_depth, &mut tex_file_writer);
    tex_file_writer.write_all(constants::LIABILITIES_FOOTER.as_bytes())
                   .unwrap();

    // Income statement
    // Write the income statement header
    tex_file_writer.write_all(format!(
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
    ).as_bytes())
                   .unwrap();

    // Income
    write_report_subsection(find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                            0,
                            max_depth,
                            &mut tex_file_writer);
    tex_file_writer.write_all(constants::INCOME_FOOTER.as_bytes())
                   .unwrap();

    // Expenses
    write_report_subsection(find_sub_tree(&root, ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES),
                            0,
                            max_depth,
                            &mut tex_file_writer);
    tex_file_writer.write_all(constants::EXPENSES_FOOTER.as_bytes())
                   .unwrap();

    // Net worth
    tex_file_writer.write_all(format!(
        "\\newpage
\\section{{Net Worth}}
Current net worth is {:8.0}
\\newpage
",
        assets_account_tree.value + liabilities_account_tree.value
    ).as_bytes())
                   .unwrap();

    // Wait for open positions processing to finish
    let investment_report_tex = rx_result.recv().unwrap();
    // And write what it produced
    tex_file_writer.write_all(investment_report_tex.as_bytes())
                   .unwrap();
}
