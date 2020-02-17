extern crate rusqlite;
#[macro_use]
extern crate rust_library;

use rusqlite::{params, Connection, LoadExtensionGuard, Statement};
use std::cmp::Ordering;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

mod constants;
mod queries;

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
    cusip: String,
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

fn escapify(s: &str) -> String {
    s.replace("%", "\\%").replace("_", "\\_").replace("&", "\\&").replace("$", "\\$")
}

fn investment_report(
    open_positions: &mut Vec<OpenPosition>, report_file_writer: &mut BufWriter<File>,
    report_type: &InvestmentReportType, date_conversion_statement: &mut Statement,
) {
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
                    InvestmentReportType::Value => report_file_writer
                        .write_all(
                            format!(
                                "{} & {:8.0} & --------\\\\\n",
                                escapify(&open_position.header.name),
                                open_position.header.quantity
                            )
                            .as_bytes(),
                        )
                        .unwrap(),
                    InvestmentReportType::MostRecentQuote => report_file_writer
                        .write_all(
                            format!("{} & --------\\\\\n", escapify(&open_position.header.name),)
                                .as_bytes(),
                        )
                        .unwrap(),
                    InvestmentReportType::CapitalGain | InvestmentReportType::TotalCapitalGain => {
                        report_file_writer
                            .write_all(
                                format!(
                                    "{} & --------\\\\\n",
                                    escapify(&open_position.header.name)
                                )
                                .as_bytes(),
                            )
                            .unwrap();
                    }
                    InvestmentReportType::AnnualizedReturn
                    | InvestmentReportType::TotalAnnualizedReturn => report_file_writer
                        .write_all(
                            format!("{} & -------\\%\\\\\n", escapify(&open_position.header.name))
                                .as_bytes(),
                        )
                        .unwrap(),
                };
            }
            Some(value) => {
                match report_type {
                    InvestmentReportType::Value => report_file_writer
                        .write_all(
                            format!(
                                "{} & {:8.0} & {:8.0}\\\\\n",
                                escapify(&open_position.header.name),
                                open_position.header.quantity,
                                value
                            )
                            .as_bytes(),
                        )
                        .unwrap(),

                    InvestmentReportType::MostRecentQuote => {
                        let timestamp = if open_position.most_recent_quote_timestamp.is_some() {
                            date_conversion_statement
                                .query_row(
                                    params![open_position
                                        .most_recent_quote_timestamp
                                        .as_ref()
                                        .unwrap()],
                                    get_result!(string),
                                )
                                .unwrap()
                        } else {
                            "None".to_string()
                        };
                        report_file_writer
                            .write_all(
                                format!(
                                    "{} & {}\\\\\n",
                                    escapify(&open_position.header.name),
                                    timestamp
                                )
                                .as_bytes(),
                            )
                            .unwrap()
                    }
                    InvestmentReportType::CapitalGain | InvestmentReportType::TotalCapitalGain => {
                        report_file_writer
                            .write_all(
                                format!(
                                    "{} & {:8.0}\\\\\n",
                                    escapify(&open_position.header.name),
                                    value
                                )
                                .as_bytes(),
                            )
                            .unwrap()
                    }
                    InvestmentReportType::AnnualizedReturn
                    | InvestmentReportType::TotalAnnualizedReturn => report_file_writer
                        .write_all(
                            format!(
                                "{} & {:7.1}\\%\\\\\n",
                                escapify(&open_position.header.name),
                                value
                            )
                            .as_bytes(),
                        )
                        .unwrap(),
                };
            }
        }
    }
}

fn main() {
    const END_DATE: usize = 1;
    const DB_FILE_INDEX: usize = END_DATE + 1;
    const REPORT_FILE_INDEX: usize = DB_FILE_INDEX + 1;
    const HOLDINGS_FILE_INDEX: usize = REPORT_FILE_INDEX + 1;
    const EXTENSIONS_FILE_INDEX: usize = HOLDINGS_FILE_INDEX + 1;
    const N_ARGS: usize = EXTENSIONS_FILE_INDEX + 1;

    // Check that the number of arguments is correct
    if env::args().count() != N_ARGS {
        panic!(
            "Incorrect number of command line arguments: {}. Should be {}.
Usage: investments endDate path_to_newcash_database path_to_report_file
    path_to_holdings_file path_to_extensions_library",
            std::env::args().count(),
            N_ARGS
        );
    }

    // Get args
    let end_date = env::args().nth(END_DATE).unwrap();
    let end_date_time = format!("{} 23:59:59", end_date);
    let holdings_file_path = env::args().nth(HOLDINGS_FILE_INDEX).unwrap();

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    // Open the output file for the reports
    let mut report_file_writer =
        BufWriter::new(File::create(env::args().nth(REPORT_FILE_INDEX).unwrap()).unwrap());

    // Load sqlite extensions, so we have math functions
    let temp = env::args().nth(EXTENSIONS_FILE_INDEX).unwrap();
    let extensions_file_path = Path::new(&temp);
    {
        let _guard = LoadExtensionGuard::new(&db).unwrap();
        db.load_extension(extensions_file_path, None).unwrap();
    }

    // Convert beginDateTime and endDateTime to julian. We will need it later for roi calculations
    let julian_end_date_time: f64;
    {
        let mut julian_conversion_statement = db.prepare(queries::JULIAN_CONVERSION_SQL).unwrap();
        julian_end_date_time = julian_conversion_statement
            .query_row(params![end_date_time], get_result!(f64))
            .unwrap();
    }

    let mut open_positions: Vec<OpenPosition> = {
        // Annualized return on investment. Note that the time interval is in days and the dates are julian.
        let roi = |begin_value: f64, end_value: f64, then: f64| -> Option<f64> {
            if begin_value == 0.0 {
                None
            } else {
                Some(
                    ((end_value / begin_value).powf(365.0 / (julian_end_date_time - then)) - 1.0)
                        * 100.0,
                )
            }
        };

        let mut open_positions: Vec<OpenPosition> = Vec::new();

        let mut open_positions_statement = db.prepare(queries::OPEN_POSITIONS_SQL).unwrap();
        let mut most_recent_zero_crossing_statement =
            db.prepare(queries::MOST_RECENT_ZERO_CROSSING_SQL).unwrap();
        let mut get_position_basis_statement = db.prepare(queries::GET_POSITION_BASIS_SQL).unwrap();
        let mut price_statement = db.prepare(queries::PRICE_SQL).unwrap();
        let mut dividend_statement = db.prepare(queries::DIVIDEND_SQL).unwrap();

        let open_positions_iter = open_positions_statement
            .query_map(params![julian_end_date_time], |row| {
                Ok(OpenPositionHeader {
                    commodity_guid: row.get(0).unwrap(),
                    symbol: row.get(1).unwrap(),
                    name: row.get(2).unwrap(),
                    cusip: row.get(3).unwrap(),
                    quantity: row.get(4).unwrap(),
                })
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
                    let (possible_post_date, quantity) = wrapped_possible_zero_crossing.unwrap();
                    remainder -= quantity;
                    if remainder.abs() < 0.1 {
                        post_date = Some(possible_post_date);
                        break;
                    }
                }
                match post_date {
                    None => panic!(
                        "Unable to find most recent zero crossing for {}",
                        open_position_header.name
                    ),
                    Some(temp) => temp,
                }
            };
            // Now compute the position basis
            let position_basis: f64 = {
                let mut basis_balance = 0.;
                let mut quantity_balance = 0.;
                let get_position_basis_iter = get_position_basis_statement
                    .query_map(
                        params![
                            open_position_header.commodity_guid,
                            most_recent_zero_crossing,
                            julian_end_date_time
                        ],
                        get_result!(f64_f64),
                    )
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
                        basis_balance = basis_balance + quantity * basis_balance / quantity_balance;
                    }
                    quantity_balance += quantity;
                }
                basis_balance
            };
            // Obtain values that are functions of price
            match price_statement.query_row(
                params![open_position_header.commodity_guid, julian_end_date_time],
                get_result!(f64_f64),
            ) {
                Ok((price, most_recent_quote_timestamp)) => {
                    let dividends = match dividend_statement.query_row(
                        params![
                            open_position_header.commodity_guid,
                            most_recent_zero_crossing,
                            julian_end_date_time
                        ],
                        get_result!(f64),
                    ) {
                        Err(_) => 0.0,
                        Ok(dividends) => dividends,
                    };
                    let current_value: f64 = open_position_header.quantity * price;
                    open_positions.push(OpenPosition {
                        header: open_position_header,
                        current_value: Some(current_value),
                        capital_gain: Some(current_value - position_basis),
                        total_gain: Some(current_value - position_basis + dividends),
                        annualized_return: roi(
                            position_basis,
                            current_value,
                            most_recent_zero_crossing,
                        ),
                        total_annualized_return: roi(
                            position_basis,
                            current_value + dividends,
                            most_recent_zero_crossing,
                        ),
                        most_recent_quote_timestamp: Some(most_recent_quote_timestamp),
                    });
                }
                Err(_) => {
                    open_positions.push(OpenPosition {
                        header: open_position_header,
                        current_value: None,
                        capital_gain: None,
                        total_gain: None,
                        annualized_return: None,
                        total_annualized_return: None,
                        most_recent_quote_timestamp: None,
                    });
                }
            }
        }
        open_positions
    };

    // Investments
    let mut date_conversion_statement = db.prepare(queries::CONVERT_JULIAN_DAY_SQL).unwrap();

    report_file_writer.write_all(constants::INVESTMENTS_HEADER.as_bytes()).unwrap();

    // Open positions subsection header
    report_file_writer.write_all(constants::OPEN_POSITIONS_SUBSECTION_HEADER.as_bytes()).unwrap();
    investment_report(
        &mut open_positions,
        &mut report_file_writer,
        &InvestmentReportType::Value,
        &mut date_conversion_statement,
    );
    report_file_writer.write_all(constants::OPEN_POSITIONS_SUBSECTION_FOOTER.as_bytes()).unwrap();

    report_file_writer
        .write_all(constants::OPEN_POSITIONS_QUOTES_SUBSECTION_HEADER.as_bytes())
        .unwrap();
    investment_report(
        &mut open_positions,
        &mut report_file_writer,
        &InvestmentReportType::MostRecentQuote,
        &mut date_conversion_statement,
    );
    report_file_writer.write_all(constants::OPEN_POSITIONS_SUBSECTION_FOOTER.as_bytes()).unwrap();

    report_file_writer.write_all(constants::CAPITAL_GAIN_SUBSECTION_HEADER.as_bytes()).unwrap();
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_HEADER.as_bytes()).unwrap();
    investment_report(
        &mut open_positions,
        &mut report_file_writer,
        &InvestmentReportType::CapitalGain,
        &mut date_conversion_statement,
    );
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_FOOTER.as_bytes()).unwrap();

    report_file_writer
        .write_all(constants::TOTAL_CAPITAL_GAIN_SUBSECTION_HEADER.as_bytes())
        .unwrap();
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_HEADER.as_bytes()).unwrap();
    investment_report(
        &mut open_positions,
        &mut report_file_writer,
        &InvestmentReportType::TotalCapitalGain,
        &mut date_conversion_statement,
    );
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_FOOTER.as_bytes()).unwrap();

    report_file_writer.write_all(constants::ANNUALIZED_GAIN_SUBSECTION_HEADER.as_bytes()).unwrap();
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_HEADER.as_bytes()).unwrap();
    investment_report(
        &mut open_positions,
        &mut report_file_writer,
        &InvestmentReportType::AnnualizedReturn,
        &mut date_conversion_statement,
    );
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_FOOTER.as_bytes()).unwrap();

    report_file_writer
        .write_all(constants::TOTAL_ANNUALIZED_GAIN_SUBSECTION_HEADER.as_bytes())
        .unwrap();
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_HEADER.as_bytes()).unwrap();
    investment_report(
        &mut open_positions,
        &mut report_file_writer,
        &InvestmentReportType::TotalAnnualizedReturn,
        &mut date_conversion_statement,
    );
    report_file_writer.write_all(constants::INVESTMENT_SUBSECTION_FOOTER.as_bytes()).unwrap();

    report_file_writer.write_all(constants::DOCUMENT_FOOTER.as_bytes()).unwrap();

    // Possibly open and write the .tsv file of open positions,
    // for use as a Google spreadsheet with which I can
    // track recent performance
    if holdings_file_path != "Nothing" {
        let mut tsv_file_writer = BufWriter::new(File::create(holdings_file_path).unwrap());
        for open_position in open_positions.iter() {
            tsv_file_writer
                .write_all(
                    format!(
                        "{}\t'{}\t{}\t{}\n",
                        open_position.header.symbol,
                        open_position.header.cusip,
                        open_position.header.name,
                        open_position.header.quantity
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
    }
}
