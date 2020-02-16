extern crate rusqlite;
extern crate rust_library;
use rust_library::constants::ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE;
use rust_library::queries::{GUID_TO_PATH_SQL, INHERITED_P_SQL};
use rust_library::{guid_to_path, inherited_p, path_to_guid};

use rusqlite::{params, Connection};
use std::env;

macro_rules! sep {
    () => {
        "\t"
    };
}

#[rustfmt::skip::macros(concat)]
#[rustfmt::skip::macros(println)]

fn main() {
    const START_DATE_INDEX: usize = 1;
    const END_DATE_INDEX: usize = START_DATE_INDEX + 1;
    const DESCRIPTION_INDEX: usize = END_DATE_INDEX + 1;
    const ACCOUNT_PATH_INDEX: usize = DESCRIPTION_INDEX + 1;
    const DB_FILE_INDEX: usize = ACCOUNT_PATH_INDEX + 1;
    const N_ARGS: usize = DB_FILE_INDEX + 1;
    const TRANSACTIONS_SQL: &str = "
        select date(post_date), t.num, t.description, t.guid
        from transactions t, splits s
        where s.account_guid = ?1
         and t.guid = s.tx_guid
         and date(t.post_date) >= ?2
         and date(t.post_date) <= ?3
         and description like ?4
        order by t.post_date, t.enter_date";
    const SPLITS_SQL: &str = "
        select account_guid, ifnull(mnemonic,''),
            ifnull(memo,''), ifnull(quantity,0.0),
            ifnull(value/nullif(quantity,0),0.),
            ifnull(value,0.)
        from (select s.account_guid, c.mnemonic, s.memo,
                s.quantity as quantity, s.value as value
              from splits s, accounts a left outer join commodities c on a.commodity_guid = c.guid
              where s.tx_guid = ?1 and a.guid = s.account_guid)";

    // Check that the number of arguments is correct
    if env::args().count() != N_ARGS {
        panic!(
            "Incorrect number of command line arguments, including program name: {}. Should  \
             be {}.\nUsage: newcashCompositeRegisterMain startDate endDate description \
             accountPath pathToDatabase",
            std::env::args().count(),
            N_ARGS
        );
    }

    // Get the args
    let start_date = env::args().nth(START_DATE_INDEX).unwrap();
    let end_date = env::args().nth(END_DATE_INDEX).unwrap();
    let description = env::args().nth(DESCRIPTION_INDEX).unwrap();
    let account_path = env::args().nth(ACCOUNT_PATH_INDEX).unwrap();

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    // Get the requested account guid and marketability
    let account_guid: String = path_to_guid(&db, &account_path);

    // Prepare statements for inherited_p and guid_to_path
    let mut inherited_p_stmt = db.prepare(INHERITED_P_SQL).unwrap();
    let mut guid_to_path_stmt = db.prepare(GUID_TO_PATH_SQL).unwrap();

    let marketable =
        inherited_p(&mut inherited_p_stmt, &account_guid, ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE);

    let mut transactions_stmt = db.prepare(TRANSACTIONS_SQL).unwrap();
    let mut splits_stmt = db.prepare(SPLITS_SQL).unwrap();

    println!(concat!("Date", sep!(), "Num", sep!(), "Description"));
    println!(concat!("", sep!(), "Account path", sep!(), "Commodity symbol", sep!(), "Split memo",
                     sep!(), "Value", sep!(), "Price", sep!(), "Quantity"));
    let transactions_iter = transactions_stmt
        .query_map(params![account_guid, start_date, end_date, description], |row| {
            Ok((row.get(0).unwrap(), row.get(1).unwrap(), row.get(2).unwrap(), row.get(3).unwrap()))
        })
        .unwrap();
    for temp in transactions_iter {
        let transaction: (String, String, String, String) = temp.unwrap();
        let (post_date, num, description, transaction_guid) = transaction;

        println!(concat!("{}", sep!(), "{}", sep!(), "{}"), post_date, num, description);
        let splits_iter = splits_stmt
            .query_map(params![transaction_guid], |row| {
                Ok((
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    row.get(2).unwrap(),
                    row.get(3).unwrap(),
                    row.get(4).unwrap(),
                    row.get(5).unwrap(),
                ))
            })
            .unwrap();
        for temp in splits_iter {
            let split: (String, String, String, f64, f64, f64) = temp.unwrap();
            let (account_guid, mnemonic, memo, quantity, price, value) = split;
            if inherited_p(
                &mut inherited_p_stmt,
                &account_guid,
                ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE,
            ) {
                println!(concat!("", sep!(), "{}", sep!(), "{}", sep!(), "{}",
                                 sep!(), "{}", sep!(), "{}", sep!(),
                                 "{}"),
                         guid_to_path(&mut guid_to_path_stmt, &account_guid),
                         mnemonic, memo, value, price, quantity);
            } else if marketable {
                println!(concat!("", sep!(), "{}", sep!(), "", sep!(), "{}",
                                 sep!(), "", sep!(), "", sep!(), "{}"),
                         guid_to_path(&mut guid_to_path_stmt, &account_guid),
                         memo, value);
            } else {
                println!(concat!("", sep!(), "{}", sep!(), "", sep!(), "{}", sep!(), "{}"),
                         guid_to_path(&mut guid_to_path_stmt, &account_guid),
                         memo, value);
            }
        }
    }
}
