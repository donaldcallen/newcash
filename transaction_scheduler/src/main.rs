extern crate rusqlite;
#[macro_use] extern crate rust_library;

use std::env;
use rusqlite::{
    Connection,
    NO_PARAMS,
    Statement,
    Error,
};
use rust_library::queries::{
    NEW_UUID_SQL,
};

fn main() {
    const DATE_INDEX: usize = 1;
    const NUM_INDEX: usize = 2;
    const DESCRIPTION_INDEX: usize = 3;
    const MINIMUM_PERIOD_INDEX: usize = 4;
    const DB_FILE_INDEX: usize = 5;
    const N_ARGS: usize = DB_FILE_INDEX + 1;
    const TEMPLATE_TRANSACTION_GUID_SQL:&str = "
        select guid from transactions
        where date(post_date)=?1
            and num=?2
            and description=?3";
    const GET_DAYS_SINCE_SQL:&str="
        select cast (round(julianday('NOW')-last_used) as integer)
        from scheduled_transactions
        where guid=?1";
    const BEGIN_TRANSACTION_SQL:&str="begin transaction";
    const COMMIT_TRANSACTION_SQL:&str="commit transaction";
    const COPY_TRANSACTION_SQL:&str="
        insert into transactions (guid, num, post_date, enter_date, description)
        select ?1, '', datetime('NOW', 'localtime'), datetime('NOW', 'localtime'), description
        from transactions where guid=?2";
    const COPY_SPLITS_SQL:&str=concat!("
        insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
        select ", constants!(NEW_UUID), ", ?1, account_guid, memo, 0, value, quantity
        from splits where tx_guid=?2");
    const INSERT_TIMESTAMP_SQL:&str="
        insert into scheduled_transactions (guid, last_used)
        values (?1, julianday('NOW'))";
    const UPDATE_TIMESTAMP_SQL:&str="
        update scheduled_transactions set last_used = julianday('NOW') where guid = ?1";

    // Check that the number of arguments is correct
    if env::args().count() != N_ARGS {
        panic!(
            "Incorrect number of command line arguments, including program name: {}. Should be {}. \
                Usage: newcashTransactionScheduler date num description minimum-period path-to-database",
            std::env::args().count(), N_ARGS
        );
    }

    // Get the args
    let date = env::args().nth(DATE_INDEX).unwrap();
    let num = env::args().nth(NUM_INDEX).unwrap();
    let description = env::args().nth(DESCRIPTION_INDEX).unwrap();
    let minimum_period:i32 = env::args().nth(MINIMUM_PERIOD_INDEX).unwrap().parse()
        .expect("Minimum period command line argument was not an integer");

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    // Prepare new_uuid_stmt
    let mut new_uuid_stmt = db.prepare(NEW_UUID_SQL).unwrap();
    
    // Get template transaction guid
    let template_transaction_guid:String = db.query_row(TEMPLATE_TRANSACTION_GUID_SQL, &[&date, &num, &description], get_result!(string))
                .expect("Template transaction not found");
    /* Is there an entry for this guid in the scheduled_transactions table?
       If so, is it more than MinimumPeriod days old?  If the answer to the first question is 'no', proceed.
       If the answer to the first is 'yes' and the second is 'yes', proceed.
       Otherwise, do nothing. This allows this program to be invoked multiple
       times by cron without inserting duplicate transactions.*/
    let maybe_days_since_last:Result<i32, Error> = db.query_row(GET_DAYS_SINCE_SQL, &[&template_transaction_guid], get_result!(i32));
    /* Do this because the expect just below consumes maybe_days_since_last, so we can't reference it again
       when need to decide whether to insert or update last_used */
    fn process_transaction<'l>(timestamp_sql:&str, new_uuid_stmt:&mut Statement<'l>, db:&Connection,
        template_transaction_guid:&String) {
        db.execute(BEGIN_TRANSACTION_SQL, NO_PARAMS).expect("Begin transaction failed");

        /* Do the copy of the template transaction within a sqlite3 transaction
           to be sure the whole thing completes without error. If it does,
           commit. If not, roll back.*/
        // Generate a guid for the new transaction
        let transaction_guid:String= new_uuid_stmt.query_row(NO_PARAMS, get_result!(string)).unwrap();

        // Copy the transaction
        db.execute(COPY_TRANSACTION_SQL, &[&transaction_guid, template_transaction_guid])
            .expect("Failed to execute transaction statement");

        // Copy the splits
        db.execute(COPY_SPLITS_SQL, &[&transaction_guid, template_transaction_guid]).expect("Failed to execute splits_stmt");

        // If we get here, record the timestamp of making the copy of the template
        db.execute(timestamp_sql, &[template_transaction_guid]).expect("Failed to execute update/insert timestamp statement");
        db.execute(COMMIT_TRANSACTION_SQL, NO_PARAMS).expect("Commit transaction failed");
    };

    if let Ok(days_since_last) = maybe_days_since_last {
        if days_since_last>minimum_period {
            process_transaction(UPDATE_TIMESTAMP_SQL, &mut new_uuid_stmt, &db, &template_transaction_guid);
        }
    } else {
        process_transaction(INSERT_TIMESTAMP_SQL, &mut new_uuid_stmt, &db, &template_transaction_guid);
    }
}
