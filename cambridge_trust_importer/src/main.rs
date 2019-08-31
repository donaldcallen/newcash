extern crate rusqlite;
#[macro_use]
extern crate rust_library;

use rusqlite::{params, Connection, Statement};
use rust_library::queries::NEW_UUID_SQL;
use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

fn main() {
    //:Assets:Bank accounts:Cambridge Trust Joint Savings
    const MBS_DISTRIBUTION: &str = "5d8cdaea96fc99db25e09791acf06bc3";
    //:Assets:Investments:Bonds and notes:Symonds Trusts:Marietta B. Symonds Trust
    const MBS_BONDS_ANCESTOR: &str = "551f2930406940096329abbe2c9777fd";
    //:Assets:Investments:Cash and cash equivalents:Symonds Trusts:Marietta B. Symonds Trust
    const MBS_CASH: &str = "555c7628aa2f714b8f77b9abaa0d2f36";
    //:Assets:Investments:Cash and cash equivalents:Symonds Trusts:Marietta B. Symonds Trust:Federated Money Market Instl
    const MBS_MONEY_MARKET: &str = "cb5dbacc36323a292f9dbc776b49c338";
    //:Assets:Investments:Equities and derivatives:Symonds Trusts:Harold Symonds Trust
    const MBS_EQUITIES_ANCESTOR: &str = "6a25bb466f9f93fdd1b9960361be1df5";
    //:Expenses:Investment:Commissions:Marietta B. Symonds Trust
    const MBS_COMMISSIONS: &str = "177352556c5b5bc32fb1768ecbc3a279";
    //:Expenses:Investment:Foreign dividend fee
    const MBS_FOREIGN_TAX: &str = "fc78147132e369b8ab1cb7870605b984";
    //:Expenses:Investment:Management fees:Symonds Trusts:Marietta B. Symonds Trust
    const MBS_MANAGEMENT_FEES: &str = "b6f71c441615a65971a1107e62d62fac";
    //:Expenses:Tax:Fiduciary (Federal)
    const MBS_FEDERAL_FIDUCIARY_TAX: &str = "3a8f6bb1ba6f72ab620d0594b39a4c11";
    //:Expenses:Tax:Fiduciary (Massachusetts)
    const MBS_STATE_FIDUCIARY_TAX: &str = "d0d576e7223b6705cef18d02b45aef5b";
    //:Income:Investments:Symonds Trusts:Capital gains:Long-term
    const MBS_LONG_TERM_CAPITAL_GAINS: &str = "4c8181fe9d080fa77a36a6f87d97104d";
    //:Income:Investments:Symonds Trusts:Capital gains:Short-term
    const MBS_SHORT_TERM_CAPITAL_GAINS: &str = "70fd801060bdf7bfb2186ccbfe4407d6";
    //:Income:Investments:Symonds Trusts:Dividends
    const MBS_DIVIDENDS_PARENT: &str = "d79fbcc69cc704018d9aebb4481dad6c";
    //:Income:Investments:Symonds Trusts:Interest
    const MBS_INTEREST_PARENT: &str = "87ad8398062e75633fa03b637b930e33";

    //:Assets:Bank accounts:Cambridge Trust Joint Savings
    const HWS_DISTRIBUTION: &str = "5d8cdaea96fc99db25e09791acf06bc3";
    //:Assets:Investments:Bonds and notes:Symonds Trusts:Harold W. Symonds Trust
    const HWS_BONDS_ANCESTOR: &str = "21f582296130ddb59d148e6838a9dcea";
    //:Assets:Investments:Cash and cash equivalents:Symonds Trusts:Harold W. Symonds Trust
    const HWS_CASH: &str = "cbad3b2c62b2ef975f2fbc4d1a55191f";
    //:Assets:Investments:Cash and cash equivalents:Symonds Trusts:Harold W. Symonds Trust:Federated Money Market Instl
    const HWS_MONEY_MARKET: &str = "8b3545e817ae9d8af6f33b9a1947d011";
    //:Assets:Investments:Equities and derivatives:Symonds Trusts:Harold Symonds Trust
    const HWS_EQUITIES_ANCESTOR: &str = "21ea36f00a270c64dd002c33e570e8cd";
    //:Expenses:Investment:Commissions:Harold W. Symonds Trust
    const HWS_COMMISSIONS: &str = "9b1c8468dee5217bbb1e0a25466580a7";
    //:Expenses:Investment:Foreign dividend fee
    const HWS_FOREIGN_TAX: &str = "fc78147132e369b8ab1cb7870605b984";
    //:Expenses:Investment:Management fees:Symonds Trusts:Harold W. Symonds Trust
    const HWS_MANAGEMENT_FEES: &str = "da5b957cd262d85f1049d36b384f9505";
    //:Expenses:Tax:Fiduciary (Federal)
    const HWS_FEDERAL_FIDUCIARY_TAX: &str = "3a8f6bb1ba6f72ab620d0594b39a4c11";
    //:Expenses:Tax:Fiduciary (Massachusetts)
    const HWS_STATE_FIDUCIARY_TAX: &str = "d0d576e7223b6705cef18d02b45aef5b";
    //:Income:Investments:Symonds Trusts:Capital gains:Long-term
    const HWS_LONG_TERM_CAPITAL_GAINS: &str = "4c8181fe9d080fa77a36a6f87d97104d";
    //:Income:Investments:Symonds Trusts:Capital gains:Short-term
    const HWS_SHORT_TERM_CAPITAL_GAINS: &str = "70fd801060bdf7bfb2186ccbfe4407d6";
    //:Income:Investments:Symonds Trusts:Dividends
    const HWS_DIVIDENDS_PARENT: &str = "d79fbcc69cc704018d9aebb4481dad6c";
    //:Income:Investments:Symonds Trusts:Interest
    const HWS_INTEREST_PARENT: &str = "87ad8398062e75633fa03b637b930e33";

    // SQL
    const BEGIN_TRANSACTION_SQL: &str = "begin transaction";
    const END_TRANSACTION_SQL: &str = "end transaction";
    const FIND_ASSET_GUID_FROM_GRANDPARENT_SQL: &str = "
    select a.guid from accounts p, accounts a, commodities c
    where p.parent_guid=?1 and a.parent_guid=p.guid and c.cusip=?2
        and a.commodity_guid=c.guid";
    const FIND_ASSET_GUID_FROM_PARENT_SQL: &str = "
    select a.guid from accounts a, commodities c
    where a.parent_guid=?1 and c.cusip=?2 and a.commodity_guid=c.guid";
    //?1 is capitalGainAccountAncestorGuid, ?2 is cusip
    const FIND_CAPITAL_GAIN_GUID_FROM_PARENT_SQL: &str = "
    select a.guid from accounts a, commodities c
    where a.parent_guid=?1 and c.cusip=?2 and a.commodity_guid=c.guid";
    const INSERT_CASH_SPLIT_SQL: &str = concat!(
                                                "
    insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
                values (",
                                                constants!(NEW_UUID),
                                                ", ?1, ?2, '', 0, ?3, 0.0)"
    );
    const INSERT_INCOME_TRANSACTION_SQL: &str = "
    insert into transactions (guid, num, post_date, enter_date, description)
                      values (?1, '',  ?2||' 12:00:00', datetime('NOW', 'localtime'), ?3)";
    // ?1 is transaction_guid, ?2 is dividends_parent_guid, ?3 is cusip, and ?4 is net_cash
    const INSERT_INCOME_TARGET_SPLIT_SQL: &str = concat!(
                                                         "
    insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
                values (",
                                                         constants!(NEW_UUID),
                                                         ", ?1,
                    (select a.guid from accounts a, commodities c where a.parent_guid=?2
                        and c.cusip=?3 and a.commodity_guid=c.guid), '', 0, -?4, 0)"
    );
    //?1 is transaction_guid, ?2 is settlement_date, ?3 is description
    const INSERT_TRADE_TRANSACTION_SQL: &str = "
    insert into transactions (guid, num, post_date, enter_date, description)
                      values (?1, '',  ?2, datetime('NOW', 'localtime'), ?3)";
    //?1 is transaction_guid, ?2 is the split account guid, ?3 is the value, ?4 is the quantity
    const INSERT_TRADE_TARGET_SPLIT_SQL: &str = concat!(
                                                        "
    insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
                values (",
                                                        constants!(NEW_UUID),
                                                        ", ?1, ?2, '', 0, ?3, ?4)"
    );

    // Indicies to command line args
    const CT_FILE_INDEX: usize = 1;
    const DB_FILE_INDEX: usize = CT_FILE_INDEX + 1;
    const N_ARGS: usize = DB_FILE_INDEX + 1;

    // Column indices into CT file
    const DESCRIPTION_INDEX: usize = 3;
    const CUSIP_INDEX: usize = 5;
    const SETTLEMENT_DATE_INDEX: usize = 7;
    const PRINCIPAL_CASH_INDEX: usize = 9;
    const NET_CASH_INDEX: usize = 10;
    const PRINCIPAL_SHARES_INDEX: usize = 20;
    const UNIT_PRICE_INDEX: usize = 22;
    const GAIN_LOSS_INDEX: usize = 30;
    const TRANSACTION_TYPE_INDEX: usize = 31;
    const NUMBER_COLUMNS_TRADE_DATA: usize = 38;

    fn convert_to_iso9601(us_date: &str) -> String {
        const YEAR_INDEX: usize = 2;
        const MONTH_INDEX: usize = 0;
        const DAY_INDEX: usize = 1;
        let split_date: Vec<&str> = us_date.split('/').collect();
        format!("{}-{}-{}",
                split_date[YEAR_INDEX], split_date[MONTH_INDEX], split_date[DAY_INDEX])
    }

    // Check that the number of arguments is correct
    if env::args().count() != N_ARGS {
        panic!(
               "Incorrect number of command line arguments: {}. Should be {}.
Usage: newcashCambridgeTrustImporter pathToCambridgeTrustFile pathToNewcashDatabase",
               std::env::args().count() - 1,
               N_ARGS - 1
        );
    }

    // Open the CT file for reading
    let ct_handle = File::open(env::args().nth(CT_FILE_INDEX).unwrap()).unwrap();
    let mut ct_reader = BufReader::new(ct_handle);
    let mut ct_buffer = String::new();

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    let account_number: i32 = {
        if let Ok(bytes) = ct_reader.read_line(&mut ct_buffer) {
            if bytes > 0 {
                let split_line: Vec<&str> = ct_buffer.split(':').collect();
                if split_line[0] == "Account Number" {
                    split_line[1].trim().parse().unwrap()
                } else {
                    panic!("First line of Cambridge Trust file does not contain account number")
                }
            } else {
                panic!("First line of Cambridge Trust file contained 0 bytes")
            }
        } else {
            panic!("Error reading Cambridge Trust file")
        }
    };

    macro_rules! choose_guid {
        ($mbs:expr, $hws:expr) => {
            match account_number {
                1265735 => $hws,
                1265743 => $mbs,
                _ => panic!("Invalid account number {}", account_number),
            }
        };
    }

    let asset_accounts_ancestor_guids = [choose_guid!(MBS_EQUITIES_ANCESTOR,
                                                      HWS_EQUITIES_ANCESTOR),
                                         choose_guid!(MBS_BONDS_ANCESTOR, HWS_BONDS_ANCESTOR)];
    let begin_transaction_stmt: RefCell<Statement> =
        RefCell::new(db.prepare(BEGIN_TRANSACTION_SQL).unwrap());
    let cash_account_guid = choose_guid!(MBS_CASH, HWS_CASH);
    let capital_gain_account_ancestor_guids =
        [choose_guid!(MBS_LONG_TERM_CAPITAL_GAINS, HWS_LONG_TERM_CAPITAL_GAINS),
         choose_guid!(MBS_SHORT_TERM_CAPITAL_GAINS, HWS_SHORT_TERM_CAPITAL_GAINS)];
    let commissions_account_guid = choose_guid!(MBS_COMMISSIONS, HWS_COMMISSIONS);
    let distribution_account_guid = choose_guid!(MBS_DISTRIBUTION, HWS_DISTRIBUTION);
    let dividends_parent_guid = choose_guid!(MBS_DIVIDENDS_PARENT, HWS_DIVIDENDS_PARENT);
    let end_transaction_stmt: RefCell<Statement> =
        RefCell::new(db.prepare(END_TRANSACTION_SQL).unwrap());
    let federal_fiduciary_tax_account_guid =
        choose_guid!(MBS_FEDERAL_FIDUCIARY_TAX, HWS_FEDERAL_FIDUCIARY_TAX);
    let state_fiduciary_tax_account_guid =
        choose_guid!(MBS_STATE_FIDUCIARY_TAX, HWS_STATE_FIDUCIARY_TAX);
    let mut find_asset_account_guid_from_grandparent_stmt =
        db.prepare(FIND_ASSET_GUID_FROM_GRANDPARENT_SQL).unwrap();
    let mut find_asset_account_guid_from_parent_stmt =
        db.prepare(FIND_ASSET_GUID_FROM_PARENT_SQL).unwrap();
    let mut find_capital_gain_account_guid_from_parent_stmt =
        db.prepare(FIND_CAPITAL_GAIN_GUID_FROM_PARENT_SQL).unwrap();
    let foreign_tax_account_guid = choose_guid!(MBS_FOREIGN_TAX, HWS_FOREIGN_TAX);
    let mut insert_cash_split_stmt = db.prepare(INSERT_CASH_SPLIT_SQL).unwrap();
    let mut insert_income_target_split_stmt = db.prepare(INSERT_INCOME_TARGET_SPLIT_SQL).unwrap();
    let insert_income_transaction_stmt: RefCell<Statement> =
        RefCell::new(db.prepare(INSERT_INCOME_TRANSACTION_SQL).unwrap());
    let insert_trade_target_split_stmt: RefCell<Statement> =
        RefCell::new(db.prepare(INSERT_TRADE_TARGET_SPLIT_SQL).unwrap());
    let mut insert_trade_transaction_stmt = db.prepare(INSERT_TRADE_TRANSACTION_SQL).unwrap();
    let interest_parent_guid = choose_guid!(MBS_INTEREST_PARENT, HWS_INTEREST_PARENT);
    let management_fees_account_guid = choose_guid!(MBS_MANAGEMENT_FEES, HWS_MANAGEMENT_FEES);
    let money_market_account_guid = choose_guid!(MBS_MONEY_MARKET, HWS_MONEY_MARKET);
    let new_guid_stmt: RefCell<Statement> = RefCell::new(db.prepare(NEW_UUID_SQL).unwrap());

    // If we know the CUSIP of the commodity paying the dividend, and we know the
    // guid of the parent of the dividend-paying accounts, then
    // select guid from accounts a, commodities c where a.parent_guid = $2
    // and c.cusip=$1 and a.commodity_guid = c.guid
    // will deliver the account guid of the income account.
    let mut process_income =
        |split_line: &Vec<&str>, description: &str, income_parent_guid: &str| {
            let cusip = split_line[CUSIP_INDEX];
            let settlement_date = convert_to_iso9601(split_line[SETTLEMENT_DATE_INDEX]);
            let net_cash: f64 = split_line[NET_CASH_INDEX].parse().unwrap();
            begin_transaction_stmt.borrow_mut()
                                  .execute(params![])
                                  .unwrap();
            // Generate a guid for the new transaction
            let transaction_guid = new_guid_stmt.borrow_mut()
                                                .query_row(params![], get_result!(string))
                                                .unwrap();
            // Insert the transaction
            insert_income_transaction_stmt.borrow_mut()
                                          .execute(params![transaction_guid,
                                                           settlement_date,
                                                           description])
                                          .unwrap();
            // And the splits
            // This statement can fail if the dividend account hasn't been set up.
            // So don't use unwrap,
            // which will fail in an uninformative way. Issue specific error
            // message in case of failure
            if insert_income_target_split_stmt.execute(params![transaction_guid,
                                                               income_parent_guid,
                                                               cusip,
                                                               net_cash])
                                              .is_err()
            {
                panic!(
                       "Unable to process income with description: {}.
         Check that the CUSIP of the commodity is correct and that the income account exists
         and points correctly to the commodity.",
                       description
                );
            }
            insert_cash_split_stmt.execute(params![transaction_guid, cash_account_guid, net_cash])
                                  .unwrap();
            end_transaction_stmt.borrow_mut()
                                .execute(params![])
                                .unwrap();
        };

    // For equity transactions, we need three splits, three accounts (apart from the capital gain
    // account needed for sales): the account for the security, the cash account,
    // and the commission account. If we know the CUSIP of the commodity we are buying,
    // and we know the guid of the grand-parent of the asset accounts (grandparent because
    // the stock accounts are usually organized in sub-accounts -- US, Europe, Asia, etc.),
    // then a simple query will deliver the account guid of the asset account for the
    // security we are buying, assuming it points correctly at the commodity.
    let mut process_trade = |split_line: &Vec<&str>, description: &str| {
        let cusip = split_line[CUSIP_INDEX];
        let settlement_date = convert_to_iso9601(split_line[SETTLEMENT_DATE_INDEX]);
        let principal_cash: f64 = split_line[PRINCIPAL_CASH_INDEX].parse().unwrap();
        let principal_shares: f64 = split_line[PRINCIPAL_SHARES_INDEX].parse().unwrap();
        let gain_loss: f64 = split_line[GAIN_LOSS_INDEX].parse().unwrap();
        let mut asset_account_guid: Option<String> = None;

        // Determine the asset account guid from ancestor guids.
        // The ancestor guids may be parents or grandparents; both are tried.
        for asset_accounts_ancestor_guid in asset_accounts_ancestor_guids.iter() {
            if let Ok(temp) = find_asset_account_guid_from_grandparent_stmt.query_row(
                params![asset_accounts_ancestor_guid, cusip],
                get_result!(string),
            ) {
                asset_account_guid = Some(temp);
                break;
            } else if let Ok(temp) = find_asset_account_guid_from_parent_stmt.query_row(
                params![asset_accounts_ancestor_guid, cusip],
                get_result!(string),
            ) {
                asset_account_guid = Some(temp);
                break;
            }
        }
        if asset_account_guid.is_none() {
            panic!("Unable to identify asset account guid for {}", description);
        }

        // If principalShares is negative, the transaction is a sale and therefore the
        // capital gain needs to be accounted for. Finding the correct account is a bit
        // complicated by the fact that capital gains can be short- or long-term.
        // For this reason, I pass two cap gains parent guids to this routine. If I find
        // a child of the first that points to a commodity with the cusip supplied in the
        // CT report, that one is used. If not, I try the second guid. If that one fails, too,
        // then the program fails.
        let unit_price: f64 = split_line[UNIT_PRICE_INDEX].parse().unwrap();
        begin_transaction_stmt.borrow_mut()
                              .execute(params![])
                              .unwrap();
        // Generate a guid for the new transaction
        let transaction_guid = new_guid_stmt.borrow_mut()
                                            .query_row(params![], get_result!(string))
                                            .unwrap();
        // Insert the transaction
        insert_trade_transaction_stmt.execute(params![transaction_guid,
                                                      settlement_date,
                                                      description.to_string()])
                                     .unwrap();
        // And the splits
        let value: f64 = principal_shares * unit_price;
        let mut borrowed_insert_trade_target_split_stmt =
            insert_trade_target_split_stmt.borrow_mut();
        borrowed_insert_trade_target_split_stmt.execute(params![transaction_guid,
                                                                asset_account_guid.as_ref()
                                                                                  .unwrap(),
                                                                value,
                                                                principal_shares])
                                               .unwrap();
        borrowed_insert_trade_target_split_stmt.execute(params![transaction_guid,
                                                                cash_account_guid,
                                                                principal_cash,
                                                                0.0])
                                               .unwrap();
        borrowed_insert_trade_target_split_stmt.execute(params![transaction_guid,
                                                                commissions_account_guid,
                                                                -principal_cash - value,
                                                                0.0])
                                               .unwrap();
        // Sale?
        if principal_shares < 0.0 {
            let capital_gain_account_guid: String = if let Ok(temp) =
                find_capital_gain_account_guid_from_parent_stmt.query_row(
                    params![capital_gain_account_ancestor_guids[0], cusip],
                    get_result!(string),
                ) {
                temp
            } else if let Ok(temp) = find_capital_gain_account_guid_from_parent_stmt.query_row(
                params![capital_gain_account_ancestor_guids[1], cusip],
                get_result!(string),
            ) {
                temp
            } else {
                panic!(
                    "Unable to identify capital gain account guid for {}.
    This error may be due to the account being non-existent, or not properly linked to its
    corresponding commodity, or because the commodity does not have a correct CUSIP.",
                    description
                );
            };
            borrowed_insert_trade_target_split_stmt.execute(params![transaction_guid,
                                                                    asset_account_guid.unwrap(),
                                                                    gain_loss,
                                                                    0.0])
                                                   .unwrap();
            borrowed_insert_trade_target_split_stmt.execute(params![transaction_guid,
                                                                    capital_gain_account_guid,
                                                                    -gain_loss,
                                                                    0.0])
                                                   .unwrap();
        };
        end_transaction_stmt.borrow_mut()
                            .execute(params![])
                            .unwrap();
    };

    let process_disbursement =
        |split_line: &Vec<&str>, description: &str, expense_account_guid: &str| {
            let settlement_date = convert_to_iso9601(split_line[SETTLEMENT_DATE_INDEX]);
            let net_cash: f64 = split_line[NET_CASH_INDEX].parse().unwrap();
            begin_transaction_stmt.borrow_mut()
                                  .execute(params![])
                                  .unwrap();
            // Generate a guid for the new transaction
            let transaction_guid = new_guid_stmt.borrow_mut()
                                                .query_row(params![], get_result!(string))
                                                .unwrap();
            // Insert the transaction
            insert_income_transaction_stmt.borrow_mut()
                                          .execute(params![transaction_guid,
                                                           settlement_date,
                                                           description])
                                          .unwrap();
            // And the splits
            let mut borrowed_insert_trade_target_split_stmt =
                insert_trade_target_split_stmt.borrow_mut();
            borrowed_insert_trade_target_split_stmt.execute(params![transaction_guid,
                                                                    expense_account_guid,
                                                                    -net_cash,
                                                                    0.0])
                                                   .unwrap();
            borrowed_insert_trade_target_split_stmt.execute(params![transaction_guid,
                                                                    cash_account_guid,
                                                                    net_cash,
                                                                    0.0])
                                                   .unwrap();
            end_transaction_stmt.borrow_mut()
                                .execute(params![])
                                .unwrap();
        };

    loop {
        ct_buffer.clear();
        match ct_reader.read_line(&mut ct_buffer) {
            Ok(bytes) => {
                if bytes > 0 {
                    let split_line: Vec<&str> = ct_buffer.split(';').collect();
                    // If we haven't reached the main part of the file,
                    // where the trades are, skip until we do.
                    if (split_line.len() != NUMBER_COLUMNS_TRADE_DATA)
                       || (split_line[1] == "ACCOUNTNUMBER")
                    {
                        continue;
                    };
                    let description = split_line[DESCRIPTION_INDEX];
                    let description_length = description.len();
                    let transaction_type = split_line[TRANSACTION_TYPE_INDEX];
                    match transaction_type {
                        "DIV" => process_income(&split_line, &description, &dividends_parent_guid),
                        "INT" => process_income(&split_line, &description, &interest_parent_guid),
                        "BUY" => process_trade(&split_line, &description),
                        "SEL" => process_trade(&split_line, &description),
                        "DIS" => match description {
                            "MANAGEMENT COMPENSATION CAMBRIDGE TRUST COMPANY "
                            | "FIDUCIARY FEE CAMBRIDGE TRUST COMPANY "
                            | "FIDUCIARY TAX SERVICE FEE"
                            | "TAX LETTER FEE" => {
                                process_disbursement(&split_line,
                                                     &description,
                                                     &management_fees_account_guid)
                            }
                            "DISTRIBUTION TO SAVINGS ACCOUNT AT CAMBRIDGE TRUST COMPANY NAME \
                             OF JOAN S ALLEN " => process_disbursement(&split_line,
                                                                       &description,
                                                                       &distribution_account_guid),
                            _ => {
                                if description_length >= 20
                                   && &description[0..20] == "FOREIGN TAX WITHHELD"
                                {
                                    process_disbursement(&split_line,
                                                         &description,
                                                         &foreign_tax_account_guid);
                                } else if description_length >= 14
                                          && &description[0..14] == "DEPOSITORY FEE"
                                {
                                    process_disbursement(&split_line,
                                                         &description,
                                                         &management_fees_account_guid);
                                } else if (description_length >= 53
                                           && &description[0..53]
                                              == "ESTIMATED FIDUCIARY INCOME TAX UNITED STATES \
                                                  TREASURY")
                                          || (description_length >= 51
                                              && &description[0..51]
                                                 == "BALANCE FIDUCIARY INCOME TAX UNITED STATES \
                                                     TREASURY")
                                {
                                    process_disbursement(&split_line,
                                                         &description,
                                                         &federal_fiduciary_tax_account_guid);
                                } else if (description_length >= 60
                                           && &description[0..60]
                                              == "ESTIMATED FIDUCIARY INCOME TAX COMMONWEALTH OF \
                                                  MASSACHUSETTS")
                                          || (description_length >= 58
                                              && &description[0..58]
                                                 == "BALANCE FIDUCIARY INCOME TAX COMMONWEALTH OF \
                                                     MASSACHUSETTS")
                                {
                                    process_disbursement(&split_line,
                                                         &description,
                                                         &state_fiduciary_tax_account_guid);
                                } else {
                                    eprintln!("Warning: unrecognized disbursement, transaction \
                                               description {}",
                                              description);
                                }
                            }
                        },
                        "ACI" => process_income(&split_line, &description, &interest_parent_guid),
                        _ => {
                            if description.trim() == "NET CASH MANAGEMENT" {
                                process_disbursement(&split_line,
                                                     &description,
                                                     &money_market_account_guid);
                            } else {
                                eprintln!("\
                                           Warning: unrecognized income transaction, transaction \
                                           description {}",
                                          description);
                            }
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
