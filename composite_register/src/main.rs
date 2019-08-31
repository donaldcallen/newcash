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
        panic!("Incorrect number of command line arguments, including program name: {}. Should  \
                be {}.\nUsage: newcashCompositeRegisterMain startDate endDate description \
                accountPath pathToDatabase",
               std::env::args().count(),
               N_ARGS);
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

    let marketable = inherited_p(&mut inherited_p_stmt,
                                 &account_guid,
                                 ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE);

    let mut transactions_stmt = db.prepare(TRANSACTIONS_SQL).unwrap();
    let mut splits_stmt = db.prepare(SPLITS_SQL).unwrap();

    println!(concat!("Date", sep!(), "Num", sep!(), "Description"));
    println!(concat!("",
                     sep!(),
                     "Account path",
                     sep!(),
                     "Commodity symbol",
                     sep!(),
                     "Split memo",
                     sep!(),
                     "Value",
                     sep!(),
                     "Price",
                     sep!(),
                     "Quantity"));
    struct Transaction {
        post_date: String,
        num: String,
        description: String,
        guid: String,
    }
    struct Split {
        account_guid: String,
        mnemonic: String,
        memo: String,
        quantity: f64,
        price: f64,
        value: f64,
    }

    let transactions_iter =
        transactions_stmt.query_map(params![account_guid, start_date, end_date, description],
                                    |row| {
                                        Ok(Transaction { post_date: row.get(0).unwrap(),
                                                         num: row.get(1).unwrap(),
                                                         description: row.get(2).unwrap(),
                                                         guid: row.get(3).unwrap() })
                                    })
                         .unwrap();
    for transaction in transactions_iter {
        let t = transaction.unwrap();
        println!(concat!("{}", sep!(), "{}", sep!(), "{}"),
                 t.post_date, t.num, t.description);
        let splits_iter = splits_stmt.query_map(params![t.guid], |row| {
                                         Ok(Split { account_guid: row.get(0).unwrap(),
                                                    mnemonic: row.get(1).unwrap(),
                                                    memo: row.get(2).unwrap(),
                                                    quantity: row.get(3).unwrap(),
                                                    price: row.get(4).unwrap(),
                                                    value: row.get(5).unwrap() })
                                     })
                                     .unwrap();
        for split in splits_iter {
            let s = split.unwrap();
            if inherited_p(&mut inherited_p_stmt,
                           &s.account_guid,
                           ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE)
            {
                println!(concat!("",
                                 sep!(),
                                 "{}",
                                 sep!(),
                                 "{}",
                                 sep!(),
                                 "{}",
                                 sep!(),
                                 "{}",
                                 sep!(),
                                 "{}",
                                 sep!(),
                                 "{}"),
                         guid_to_path(&mut guid_to_path_stmt, &s.account_guid),
                         s.mnemonic,
                         s.memo,
                         s.value,
                         s.price,
                         s.quantity);
            } else if marketable {
                println!(concat!("",
                                 sep!(),
                                 "{}",
                                 sep!(),
                                 "",
                                 sep!(),
                                 "{}",
                                 sep!(),
                                 "",
                                 sep!(),
                                 "",
                                 sep!(),
                                 "{}"),
                         guid_to_path(&mut guid_to_path_stmt, &s.account_guid),
                         s.memo,
                         s.value);
            } else {
                println!(concat!("", sep!(), "{}", sep!(), "", sep!(), "{}", sep!(), "{}"),
                         guid_to_path(&mut guid_to_path_stmt, &s.account_guid),
                         s.memo,
                         s.value);
            }
        }
    }
}
