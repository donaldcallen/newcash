extern crate rusqlite;
#[macro_use]
extern crate rust_library;

use rusqlite::{params, Connection, Statement};
use rust_library::queries::NEW_UUID_SQL;
use std::env;
use std::io;
use std::io::{prelude::*, BufReader};

// Types
struct PerAccountGuids {
    asset_parents: [&'static str; 3],
    cash_account: &'static str,
    commissions_account: &'static str,
    dividends_parent: &'static str,
    sweep_account: &'static str,
}

struct Statements<'l> {
    begin_transaction: Statement<'l>,
    create_commodity_account: Statement<'l>,
    end_transaction: Statement<'l>,
    get_account_guid: Statement<'l>,
    get_commodity_info: Statement<'l>,
    insert_account: Statement<'l>,
    insert_split: Statement<'l>,
    insert_transaction: Statement<'l>,
    new_guid: Statement<'l>,
}

// Guids
//:Assets:Investments:Bonds and notes:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard
const DCA_TRUST_BONDS_AND_NOTES: &str = "b21804cd05b09d83ffa9a1c444297b2d";
//:Assets:Investments:Equities and derivatives:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard:International
const DCA_TRUST_EQUITIES_INTERNATIONAL: &str = "839b3e24f1edb90ebd76cf463375a47d";
//:Assets:Investments:Equities and derivatives:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard:United States
const DCA_TRUST_EQUITIES_US: &str = "d934624c13b1ceae687fb03c69b8cfa2";
//:Assets:Investments:Cash and cash equivalents:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard
const DCA_TRUST_CASH: &str = "95ac1e5b9762b96d7bd87b7758fb3137";
//:Expenses:Investment:Commissions:Vanguard:Donald C. Allen 2003 Revocable Trust
const DCA_TRUST_COMMISSIONS: &str = "814341b0ddab766c884bf89e8f263bb5";
//:Assets:Investments:Bonds and notes:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard:Vanguard Federal Money Market Fund
const DCA_TRUST_SWEEP_ACCOUNT: &str = "ac56147de1e64dc1f1de6f27cd009db7";

//:Assets:Investments:Bonds and notes:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard
const JSA_TRUST_BONDS_AND_NOTES: &str = "8e93bac71299464a2964db225f9902bf";
//:Assets:Investments:Equities and derivatives:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard:Europe
const JSA_TRUST_EQUITIES_EUROPE: &str = "c765b367a40099a2bf6c3ef10ed1b901";
//:Assets:Investments:Equities and derivatives:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard:United States
const JSA_TRUST_EQUITIES_US: &str = "c98dcdcb469ce66afb044c3b89293979";
//:Assets:Investments:Cash and cash equivalents:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard
const JSA_TRUST_CASH: &str = "4ab9d5c9999a18736f4c495210cc3c02";
//:Expenses:Investment:Commissions:Vanguard:Joan S. Allen 2003 Revocable Trust
const JSA_TRUST_COMMISSIONS: &str = "db543a9cf6e87e5b2f15223eeb6ee17e";
//:Assets:Investments:Bonds and notes:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard:Vanguard Federal Money Market Fund
const JSA_TRUST_SWEEP_ACCOUNT: &str = "f3482f943e1fda9bff85db73e5152fa7";

//:Assets:Investments:Bonds and notes:Tax-deferred:Don:Vanguard Individual IRA
const DCA_INDIVIDUAL_IRA_BONDS_AND_NOTES: &str = "6a77cc0d486761ff737abc01a9b68aff";
//:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Individual IRA:International
const DCA_INDIVIDUAL_IRA_EQUITIES_INTERNATIONAL: &str = "5eb50f9c8bb79e5154d6247c8fdcb753";
//:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Individual IRA:United States
const DCA_INDIVIDUAL_IRA_EQUITIES_US: &str = "468133b246bee4019dc024295fc73731";
//:Assets:Investments:Cash and cash equivalents:Tax-deferred:Don:Vanguard Individual IRA
const DCA_INDIVIDUAL_IRA_CASH: &str = "b75cb26cc20ce1ea928fa0036185ca03";
//:Expenses:Investment:Commissions:Vanguard:DCA Individual IRA
const DCA_INDIVIDUAL_IRA_COMMISSIONS: &str = "0b348120ab9a07e5e2a080f39fee6d46";
//:Assets:Investments:Bonds and notes:Tax-deferred:Don:Vanguard Individual IRA:Vanguard Federal Money Market Fund
const DCA_INDIVIDUAL_IRA_SWEEP_ACCOUNT: &str = "8138984074b71298b263c6a0882d2887";

//:Assets:Investments:Bonds and notes:Tax-deferred:Don:Vanguard Inherited IRA
const DCA_INHERITED_IRA_BONDS_AND_NOTES: &str = "dff45b6049e8d8ed9cd9301c71c1db23";
//:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Inherited IRA:International
const DCA_INHERITED_IRA_EQUITIES_INTERNATIONAL: &str = "9c8c5029318c2013a5a8e45526d01284";
//:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Inherited IRA:United States
const DCA_INHERITED_IRA_EQUITIES_US: &str = "7415960ea7444842ed87f42cfbd49e3c";
//:Assets:Investments:Cash and cash equivalents:Tax-deferred:Don:Vanguard Inherited IRA
const DCA_INHERITED_IRA_CASH: &str = "f301ade58f8adff68ee021a03e11e29f";
//:Expenses:Investment:Commissions:Vanguard:DCA Inherited IRA
const DCA_INHERITED_IRA_COMMISSIONS: &str = "025585fbe2ff1f1c8309d23918a6376e";
//:Assets:Investments:Bonds and notes:Tax-deferred:Don:Vanguard Inherited IRA:Vanguard Federal Money Market Fund
const DCA_INHERITED_IRA_SWEEP_ACCOUNT: &str = "a0cc50982c7c2da89f8629bd36446216";

const DEFAULT_ASSET_PARENT_INDEX: usize = 2;

//:Income:Investments:Taxable:Dividends
const TAXABLE_DIVIDENDS: &str = "b4049ae08bd9f4bc9826ccfa9da503b5";
//:Income:Investments:Tax-deferred:Dividends
const TAX_DEFERRED_DIVIDENDS: &str = "5c6db83acf4c0c1e30183eba80c030cd";

//:Expenses:Tax:Foreign
const FOREIGN_TAX_EXPENSE_GUID: &str = "3fcabaaef90cc0519f2df3b4e19eb06e";
//:Expenses:Investment:Management fees:ADR custody fees
const ADR_CUSTODY_FEE_EXPENSE_GUID: &str = "8b2720407446a7365fe30810b7ccb09a";
//:Unspecified
const UNSPECIFIED_ACCOUNT_GUID: &str = "b1491c8019a58916d38e51c817741008";

// SQL
const BEGIN_TRANSACTION: &str = "begin transaction";
const END_TRANSACTION: &str = "end transaction";
// ?1 is target_guid, ?2 is name, ?3 is parent_guid, ?4 is commodity_guid
const INSERT_ACCOUNT: &str = "
    insert into accounts (guid, name, parent_guid, commodity_guid, code, description, flags)
                            values (?1, ?2, ?3, ?4, '', '', 0)";
//?1 is transaction, ?2 is settlement_date, ?3 is description
const INSERT_TRANSACTION: &str = "
    insert into transactions (guid, num, post_date, enter_date, description) 
                            values (?1, '',  ?2||' 12:00:00', datetime('NOW', 'localtime'), ?3)";
// ?1 is the transaction guid, ?2 is the account guid, ?3 is value, ?4 is the quantity
const INSERT_SPLIT: &str = concat!(
                                   "
    insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                values (",
                                   constants!(NEW_UUID),
                                   ", ?1, ?2, '', 0, ?3, ?4)"
);
// ?1 is the symbol
const GET_COMMODITY_INFO: &str = "select guid, ifnull(flags,0) from commodities where mnemonic=?1";
// ?1 is parent guid, ?2 is the symbol
const FIND_ACCOUNT_GUID: &str = "
    select a.guid
    from accounts a, commodities c
    where parent_guid=?1
        and c.guid = a.commodity_guid
        and c.mnemonic = ?2";
// ?1 is the commodity guid, ?2 is the symbol, ?3 is the commodity name
const CREATE_COMMODITY_ACCOUNT: &str = "
    insert into commodities (guid, mnemonic, fullname, flags)
                    values (?1, ?2, ?3, 0)";

// Indicies to command line args
const ACCOUNT_NUMBER_INDEX: usize = 1;
const DB_FILE_INDEX: usize = ACCOUNT_NUMBER_INDEX + 1;
const N_ARGS: usize = DB_FILE_INDEX + 1;

// Column indices into .csv file
const SETTLEMENT_DATE_INDEX: usize = 1;
const TRANSACTION_TYPE_INDEX: usize = 2;
const NAME_INDEX: usize = 4;
const SYMBOL_INDEX: usize = 5;
const QUANTITY_INDEX: usize = 6;
const COMMISSION_INDEX: usize = 9;
const AMOUNT_INDEX: usize = 10;

fn string_to_number(s: &str) -> f64 {
    if s == "" {
        0.
    } else {
        s.parse().unwrap()
    }
}

fn insert_transaction(settlement_date: &str, description: &str, target: &str, amount: f64,
                      quantity: f64, commission: f64, close_p: bool,
                      per_account_guids: &PerAccountGuids, statements: &mut Statements) {
    statements.begin_transaction.execute(params![]).unwrap();
    // Generate a guid for the new transaction
    let transaction_guid = statements.new_guid
                                     .query_row(params![], get_result!(string))
                                     .unwrap();
    // Insert the transaction
    statements.insert_transaction
              .execute(params![transaction_guid, settlement_date, description])
              .unwrap();
    // And the splits
    // quantity has correct sign, but amount is from the cash account point of view
    statements.insert_split
              .execute(params![transaction_guid, target, -amount, quantity])
              .unwrap();
    // Commission is always positive, therefore it is stated from the perspective of the expense account. So here it always needs
    // to be subtracted from this split's amount, which is from the perspective of the cash account.
    statements.insert_split
              .execute(params![transaction_guid,
                               per_account_guids.cash_account,
                               amount - commission,
                               0.0])
              .unwrap();
    if commission != 0.0 {
        // Create a split for the commission expense
        statements.insert_split
                  .execute(params![transaction_guid,
                                   per_account_guids.commissions_account,
                                   commission,
                                   0.0])
                  .unwrap();
    }
    if close_p {
        // Insert splits for capital gain
        statements.insert_split
                  .execute(params![transaction_guid, target, 0.0, 0.0])
                  .unwrap();
        statements.insert_split
                  .execute(params![transaction_guid, UNSPECIFIED_ACCOUNT_GUID, 0.0, 0.0])
                  .unwrap();
    }
    statements.end_transaction.execute(params![]).unwrap();
}

fn get_target_and_insert_transaction(name: &str, settlement_date: &str, description: &str,
                                     parent_guid: &str, symbol: &str, amount: f64,
                                     quantity: f64, commission: f64, close_p: bool,
                                     per_account_guids: &PerAccountGuids,
                                     statements: &mut Statements) {
    // Get commodity guid and die if it doesn't exist
    let commodity_guid: String =
        match statements.get_commodity_info
                        .query_row(params![symbol], get_result!(string_i32))
        {
            Err(_) => panic!("get_target_and_process_transaction: failed to find commodity for \
                              {}, {}",
                             symbol, name),
            Ok((commodity_guid, _)) => commodity_guid,
        };
    let target_guid = if let Some(target_guid) = find_account_guid(symbol, parent_guid, statements)
    {
        target_guid
    } else {
        let target_guid = statements.new_guid
                                    .query_row(params![], get_result!(string))
                                    .unwrap();
        println!("processTransaction: creating account for {}, {}",
                 name, symbol);
        statements.insert_account
                  .execute(params![target_guid, name, parent_guid, commodity_guid])
                  .unwrap();
        target_guid
    };
    insert_transaction(settlement_date,
                       description,
                       target_guid.as_str(),
                       amount,
                       quantity,
                       commission,
                       close_p,
                       per_account_guids,
                       statements);
}

fn find_account_guid(symbol: &str, parent_guid: &str, statements: &mut Statements)
                     -> Option<String> {
    if let Ok(guid) = statements.get_account_guid
                                .query_row(params![parent_guid, symbol], get_result!(string))
    {
        Some(guid)
    } else {
        None
    }
}

fn find_asset_guid(symbol: &str, parent_guids: &[&str; 3], statements: &mut Statements)
                   -> Option<String> {
    // Search parents for a child asset that matches the symbol
    for parent_guid in parent_guids.iter() {
        match find_account_guid(symbol, &parent_guid, statements) {
            None => continue,
            result => return result,
        }
    }
    None
}

fn create_commodity(symbol: &str, name: &str, statements: &mut Statements) -> String {
    // No commodity present, create it
    println!("find_asset_guid: creating commodity for {}, {}",
             symbol, name);
    let commodity_guid: String = statements.new_guid
                                           .query_row(params![], get_result!(string))
                                           .unwrap();
    statements.create_commodity_account
              .execute(params![commodity_guid, symbol, name])
              .unwrap();
    commodity_guid
}

fn main() {
    // Check that the number of arguments is correct
    if env::args().count() != N_ARGS {
        panic!(
               "Incorrect number of command line arguments: {}. Should 
    be {}. Usage: newcashVanguardImporter pathToVanguardFile pathToNewcashDatabase",
               std::env::args().count() - 1,
               N_ARGS - 1
        );
    }

    // Set up the Vanguard file, which is supplied via stdin, for reading
    let vngd_handle = io::stdin();
    let mut vngd_reader = BufReader::new(vngd_handle);
    let mut vngd_buffer = String::new();

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    let account_number: i32 = env::args().nth(ACCOUNT_NUMBER_INDEX)
                                         .unwrap()
                                         .parse()
                                         .unwrap();

    // Get guids that are a function of the account number
    let per_account_guids = match account_number {
        18477440 => PerAccountGuids { asset_parents: [DCA_TRUST_BONDS_AND_NOTES,
                                                      DCA_TRUST_EQUITIES_INTERNATIONAL,
                                                      DCA_TRUST_EQUITIES_US],
                                      cash_account: DCA_TRUST_CASH,
                                      commissions_account: DCA_TRUST_COMMISSIONS,
                                      dividends_parent: TAXABLE_DIVIDENDS,
                                      sweep_account: DCA_TRUST_SWEEP_ACCOUNT },
        10792723 => PerAccountGuids { asset_parents: [JSA_TRUST_BONDS_AND_NOTES,
                                                      JSA_TRUST_EQUITIES_EUROPE,
                                                      JSA_TRUST_EQUITIES_US],
                                      cash_account: JSA_TRUST_CASH,
                                      commissions_account: JSA_TRUST_COMMISSIONS,
                                      dividends_parent: TAXABLE_DIVIDENDS,
                                      sweep_account: JSA_TRUST_SWEEP_ACCOUNT },
        66996984 => PerAccountGuids { asset_parents: [DCA_INDIVIDUAL_IRA_BONDS_AND_NOTES,
                                                      DCA_INDIVIDUAL_IRA_EQUITIES_INTERNATIONAL,
                                                      DCA_INDIVIDUAL_IRA_EQUITIES_US],
                                      cash_account: DCA_INDIVIDUAL_IRA_CASH,
                                      commissions_account: DCA_INDIVIDUAL_IRA_COMMISSIONS,
                                      dividends_parent: TAX_DEFERRED_DIVIDENDS,
                                      sweep_account: DCA_INDIVIDUAL_IRA_SWEEP_ACCOUNT },
        36750678 => PerAccountGuids { asset_parents: [DCA_INHERITED_IRA_BONDS_AND_NOTES,
                                                      DCA_INHERITED_IRA_EQUITIES_INTERNATIONAL,
                                                      DCA_INHERITED_IRA_EQUITIES_US],
                                      cash_account: DCA_INHERITED_IRA_CASH,
                                      commissions_account: DCA_INHERITED_IRA_COMMISSIONS,
                                      dividends_parent: TAX_DEFERRED_DIVIDENDS,
                                      sweep_account: DCA_INHERITED_IRA_SWEEP_ACCOUNT },
        _ => panic!("Invalid account number {}", account_number),
    };

    let mut statements =
        Statements { begin_transaction: db.prepare(BEGIN_TRANSACTION).unwrap(),
                     create_commodity_account: db.prepare(CREATE_COMMODITY_ACCOUNT).unwrap(),
                     end_transaction: db.prepare(END_TRANSACTION).unwrap(),
                     get_account_guid: db.prepare(FIND_ACCOUNT_GUID).unwrap(),
                     get_commodity_info: db.prepare(GET_COMMODITY_INFO).unwrap(),
                     insert_account: db.prepare(INSERT_ACCOUNT).unwrap(),
                     insert_split: db.prepare(INSERT_SPLIT).unwrap(),
                     insert_transaction: db.prepare(INSERT_TRANSACTION).unwrap(),
                     new_guid: db.prepare(NEW_UUID_SQL).unwrap() };

    loop {
        vngd_buffer.clear();
        match vngd_reader.read_line(&mut vngd_buffer) {
            Ok(bytes) => {
                if bytes > 0 {
                    let split_line: Vec<&str> = vngd_buffer.split(',').collect();
                    let settlement_date: &str = split_line[SETTLEMENT_DATE_INDEX];
                    let name: &str = split_line[NAME_INDEX];
                    let transaction_type: &str = split_line[TRANSACTION_TYPE_INDEX];
                    let mut quantity: f64 = string_to_number(split_line[QUANTITY_INDEX]);
                    let commission: f64 = string_to_number(split_line[COMMISSION_INDEX]);
                    let amount: f64 = string_to_number(split_line[AMOUNT_INDEX]);
                    let mut symbol: &str = split_line[SYMBOL_INDEX];
                    if symbol.len() == 0 {
                        if name == "VANGUARD FEDERAL MONEY MARKET FUND" {
                            // For some odd reason, Vanguard omits this symbol from its .csv files
                            symbol = "VMFXX";
                            // And also omits the quantity in re-investment transactions
                            if transaction_type == "Reinvestment" {
                                quantity = -amount;
                            }
                        }
                    }

                    match transaction_type {
                        "Sell" | "Sell to close" | "Sell to open" | "Assignment" | "Expired"
                        | "Buy" | "Buy to open" | "Buy to close" => {
                            // Is this an option transaction?
                            if (&(name[0..4]) == "CALL") || (&(name[0..3]) == "PUT") {
                                quantity = quantity * 100.;
                            }
                            if symbol.len() == 0 {
                                panic!("No symbol supplied line {}", vngd_buffer);
                            }
                            let asset_guid = if let Some(asset_guid) =
                                find_asset_guid(symbol,
                                                &per_account_guids.asset_parents,
                                                &mut statements)
                            {
                                asset_guid
                            } else if (transaction_type == "Sell to open")
                                      || (transaction_type == "Buy to open")
                                      || (transaction_type == "Buy")
                            {
                                // Make sure commodity exists
                                let commodity_guid: String =
                                    match statements.get_commodity_info
                                                    .query_row(params![symbol],
                                                               get_result!(string_i32))
                                    {
                                        Err(_) => create_commodity(symbol, name, &mut statements),
                                        Ok((commodity_guid, _)) => commodity_guid,
                                    };
                                // Create asset account under default parent
                                println!("Creating asset account for {}, {}", symbol, name);
                                let asset_account_guid: String =
                                    statements.new_guid
                                              .query_row(params![], get_result!(string))
                                              .unwrap();
                                statements.insert_account
                                          .execute(params![asset_account_guid,
                                                           name,
                                                           per_account_guids.asset_parents
                                                               [DEFAULT_ASSET_PARENT_INDEX],
                                                           commodity_guid])
                                          .unwrap();
                                asset_account_guid
                            } else {
                                panic!("Failed to find asset account for closing transaction for \
                                        {}, {}, {}",
                                       symbol, name, transaction_type)
                            };
                            let description: String = format!("{} {}", transaction_type, name);
                            insert_transaction(settlement_date,
                                               description.as_str(),
                                               asset_guid.as_str(),
                                               amount + commission,
                                               quantity,
                                               commission,
                                               transaction_type == "Sell"
                                               || transaction_type == "Sell to close"
                                               || transaction_type == "Assignment"
                                               || transaction_type == "Expired"
                                               || transaction_type == "Buy to close",
                                               &per_account_guids,
                                               &mut statements);
                        }
                        "Dividend (adjustment)" | "Dividend" => {
                            let description: String = format!("{} dividend", name);
                            get_target_and_insert_transaction(name,
                                                              settlement_date,
                                                              description.as_str(),
                                                              &per_account_guids.dividends_parent,
                                                              symbol,
                                                              amount,
                                                              0.0,
                                                              0.0,
                                                              false,
                                                              &per_account_guids,
                                                              &mut statements);
                        }
                        "Sweep out" | "Sweep in" => {
                            insert_transaction(settlement_date,
                                               transaction_type,
                                               per_account_guids.sweep_account,
                                               amount,
                                               -amount,
                                               0.0,
                                               false,
                                               &per_account_guids,
                                               &mut statements);
                        }
                        "Reinvestment" => {
                            let asset_guid = if let Some(asset_guid) =
                                find_asset_guid(symbol,
                                                &per_account_guids.asset_parents,
                                                &mut statements)
                            {
                                asset_guid
                            } else {
                                panic!("Failed to find asset account for reinvestment \
                                        transaction for {}, {}, {}",
                                       symbol, name, transaction_type);
                            };
                            let description: String = format!("Re-invest {} dividend", name);
                            insert_transaction(settlement_date,
                                               description.as_str(),
                                               asset_guid.as_str(),
                                               amount,
                                               quantity,
                                               commission,
                                               false,
                                               &per_account_guids,
                                               &mut statements);
                        }
                        "Withholding" => {
                            let description: String = format!("Foreign tax withheld ({})", name);
                            insert_transaction(settlement_date,
                                               description.as_str(),
                                               FOREIGN_TAX_EXPENSE_GUID,
                                               amount,
                                               0.,
                                               0.0,
                                               false,
                                               &per_account_guids,
                                               &mut statements);
                        }
                        "Fee" => {
                            let description: String = format!("ADR custody fee ({})", name);
                            insert_transaction(settlement_date,
                                               description.as_str(),
                                               ADR_CUSTODY_FEE_EXPENSE_GUID,
                                               amount,
                                               0.,
                                               0.,
                                               false,
                                               &per_account_guids,
                                               &mut statements);
                        }
                        _ => {
                            eprintln!("Unable to process transaction of type {}, settlement date \
                                       {}, name {}",
                                      transaction_type, settlement_date, name);
                        }
                    }
                } else {
                    break;
                }
            }
            Err(the_error) => panic!("Error in read_line: {:?}", the_error.kind()),
        }
    }
}
