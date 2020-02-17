extern crate rusqlite;
#[macro_use]
extern crate rust_library;

use rusqlite::{params, Connection, Statement};
use rust_library::constants::{
    ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS, ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES,
    ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME, ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES,
    ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE, ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK,
    ACCOUNT_FLAG_HIDDEN, ACCOUNT_FLAG_NOCHILDREN, ACCOUNT_FLAG_PERMANENT, ACCOUNT_FLAG_PLACEHOLDER,
};
use rust_library::guid_to_path;
use rust_library::queries::GUID_TO_PATH_SQL;
use std::env;

// Types
struct Globals<'a> {
    get_possible_commodity_guid: Statement<'a>,
    new_guid: Statement<'a>,
    insert_new_commodity: Statement<'a>,
    link_to_commodity: Statement<'a>,
    verify_commodity_guid: Statement<'a>,
    nullify_commodity_guid: Statement<'a>,
    check_commodity_name: Statement<'a>,
    count_transactions: Statement<'a>,
    find_children: Statement<'a>,
    check_quantities: Statement<'a>,
    fix_quantities: Statement<'a>,
    check_money_market: Statement<'a>,
    fix_money_market_quantities: Statement<'a>,
}

struct Account {
    name: String,
    path: String,
    guid: String,
    commodity_guid: String,
    flags: i32,
}

impl<'a> Globals<'a> {
    fn fix_missing_commodity(&mut self, account: &Account) {
        let new_commodity_guid = self.new_guid.query_row(params![], get_result!(string)).unwrap();
        // Create new commodity
        self.insert_new_commodity.execute(params![new_commodity_guid, account.name]).unwrap();
        // And link the account to the new commodity
        self.link_to_commodity.execute(params![new_commodity_guid, account.guid]).unwrap();
    }
    fn check_and_repair_commodity_link(&mut self, account: &Account) {
        if account.commodity_guid == "" {
            let possible_commodity_guid = self
                .get_possible_commodity_guid
                .query_row(params![account.name], get_result!(string));
            if possible_commodity_guid.is_err() {
                // No commodity exists with the same name as the account.
                // Create one and link the account to it.
                println!(
                    "{} requires a link to a commodity but doesn't have one. A commodity
with the same name as the account does not exist. One will be created
with the symbol **unknown** and the account will be linked to it. If
you wish to get quotes for this commodity, you will need to fix the
symbol in Newcash.",
                    account.path
                );
                self.fix_missing_commodity(account);
            } else {
                println!(
                    "{} requires a link to a commodity but doesn't have one.
A commodity with the same name as the account does exist. The account will be linked to it.",
                    account.name
                );
                self.link_to_commodity
                    .execute(params![possible_commodity_guid.unwrap(), account.guid])
                    .unwrap();
            }
        } else {
            // This account has a commodity. Check to be sure that the guid is valid.
            // If not, set the account's commodity_guid to NULL and process this account again.
            match self
                .verify_commodity_guid
                .query_row(params![account.commodity_guid], get_result!(string))
            {
                Err(_) => {
                    println!(
                        "{} requires a commodity link and has one, but the commodity guid is
invalid. Creating a new commodity with the same name as the account and linking the account to it.",
                        account.path
                    );
                    self.fix_missing_commodity(account);
                }
                Ok(_) => {
                    // Warn if the commodity name is not the same as the account name
                    if self
                        .check_commodity_name
                        .query_row(params![account.guid], get_result!(i32))
                        .unwrap()
                        == 0
                    {
                        println!(
                            "{} requires a commodity link and has one,
but the commodity name is not the same as the account name.  Is this intentional?",
                            account.path
                        );
                    }
                }
            }
        }
    }

    fn walk_account_tree(&mut self, account: &Account, ancestor_flags: i32) {
        // Placeholder?
        if (account.flags & ACCOUNT_FLAG_PLACEHOLDER) == 0 {
            // No. Is this account an asset?
            if (ancestor_flags & ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS) != 0 {
                // Yes. Is is marketable?
                if (ancestor_flags & ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE) != 0 {
                    // Yes
                    self.check_and_repair_commodity_link(account);
                    // Is this a money-market fund?
                    if self.check_money_market.query_row(params!(account.guid), get_result!(i32)).unwrap() != 0 {
                        // Make sure all money market account splits have identical quantities and values
                        self.fix_money_market_quantities.execute(params![account.guid]).unwrap();
                    }
                } else {
                    // Not a marketable asset
                    if account.commodity_guid != "" {
                        // This account is a non-marketable asset and has a non-null commodity guid.
                        // Set to NULL. Non-marketable accounts should not point to commodities.
                        println!(
                            "{} is a non-marketable asset, but it is associated with a
        commodity. Removing the association by setting the account's commodity link to NULL.",
                            account.path
                        );
                        self.nullify_commodity_guid.execute(params![account.guid]).unwrap();
                    }
                    // Make sure the quantity is zero. Should not be otherwise for a non-marketable asset.
                    if self
                        .check_quantities
                        .query_row(params![account.guid], get_result!(i32))
                        .unwrap()
                        > 0
                    {
                        println!(
                            "{} is a non-marketable asset account but has splits with \
                             non-zero quantities. These will be fixed.",
                            account.path
                        );
                        self.fix_quantities.execute(params![account.guid]).unwrap();
                    }
                }
            } else {
                // This account is not an asset. Make sure it doesn't point to a commodity,
                // unless it is an Income account and inherits the descendents-need-commodity property.
                // Does it need a commodity link?
                if (ancestor_flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK) != 0 {
                    // Yes. Is it an income account?
                    if (ancestor_flags & ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME) != 0 {
                        // Yes
                        self.check_and_repair_commodity_link(account);
                    } else {
                        println!(
                            "The account {} is not an Asset or Income account, but inherits \
                                  the
    'needs commodity' property. This should not be possible and
    indicates a bug in Newcash or in the Verifier. Please report to Don Allen.",
                            account.path
                        );
                    }
                } else {
                    // The account is not an asset and does not have needs-commodity-link property.
                    // It should not have a commodity link. Remove it if it does.
                    if account.commodity_guid != "" {
                        println!(
                            "{} is not an asset, and doesn't have the needs-commodity-link
    property, but it is associated with a commodity. Removing the association by setting
    the account's commodity link to NULL.",
                            account.path
                        );
                        self.nullify_commodity_guid.execute(params![account.guid]).unwrap();
                    }
                    // Make sure it doesn't inherit the marketable property
                    if (ancestor_flags & ACCOUNT_FLAG_DESCENDENTS_ARE_MARKETABLE) != 0 {
                        println!(
                            "{} is designated marketable, but is not an asset account. This
    should not be possible and is indicative of a Newcash bug. Please report this to Don Allen.",
                            account.path
                        );
                    }
                }
                // Make sure the quantity is zero. Should not be otherwise for a non-asset.
                if self.check_quantities.query_row(params![account.guid], get_result!(i32)).unwrap()
                    > 0
                {
                    println!(
                        "{} is not an asset account but has splits with non-zero \
                         quantities. These will be fixed.",
                        account.path
                    );
                    self.fix_quantities.execute(params![account.guid]).unwrap();
                }
            }
        } else
        // This account is a place-holder. Verify that it has no transactions.
        // Warn the user if that is not true.
        if self
            .count_transactions
            .query_row(params![account.guid], get_result!(i32))
            .unwrap()
            > 0
        {
            println!(
                "{} is a placeholder account, but it has transactions. You should
    re-assign them to an appropriate account with Newcash",
                account.path
            );
        };
        // Now do the children of this account
        let mut children: Vec<Account> = Vec::new();
        {
            let children_iter = self
                .find_children
                .query_map(params![account.guid], get_result!(string_string_string_i32))
                .unwrap();
            for child_info_result in children_iter {
                let (name, guid, commodity_guid, flags) = child_info_result.unwrap();
                let mut path = account.path.clone();
                path.push(':');
                path.push_str(name.as_str());

                let child = Account {
                    name,
                    path,
                    guid,
                    commodity_guid,
                    flags,
                };
                children.push(child);
            }
        }
        for child in children.iter() {
            self.walk_account_tree(&child, ancestor_flags | account.flags);
        }
    }
}

fn main() {
    const DB_FILE_INDEX: usize = 1;
    const N_ARGS: usize = DB_FILE_INDEX + 1;

    // Check that the number of arguments is correct
    if env::args().count() != N_ARGS {
        panic!(
            "Incorrect number of command line arguments: {}. Should be {}.
Usage: newcashverifier pathToDatabase",
            std::env::args().count(),
            N_ARGS
        );
    }

    // Open the database
    let db = Connection::open(env::args().nth(DB_FILE_INDEX).unwrap()).unwrap();

    // Make sure all the required tables are present and have correct flags
    {
        let mut check_table_stmt =
            db.prepare("select count(*) from sqlite_master where tbl_name=?1").unwrap();
        let mut check_table = |table: &'static str, create_sql: &'static str| {
            if check_table_stmt.query_row(params![table], get_result!(i32)).unwrap() == 0 {
                println!("The {} table is missing. Creating ...", table);
                db.execute(create_sql, params![]).unwrap();
            };
        };
        check_table(
            "accounts",
            "CREATE TABLE accounts (guid text PRIMARY KEY NOT NULL,
                                            name text NOT NULL,
                                            parent_guid text REFERENCES accounts (guid),
                                            commodity_guid text REFERENCES commodities (guid),
                                            code text,
                                            description text,
                                            flags integer)",
        );
        check_table(
            "book",
            "CREATE TABLE book (root_account_guid text NOT NULL
                REFERENCES accounts (guid), name text)",
        );
        check_table(
            "prices",
            "CREATE TABLE prices (guid text PRIMARY KEY NOT NULL,
                                          commodity_guid text NOT NULL
                                            REFERENCES commodities (guid),
                                          timestamp text NOT NULL
                                            CHECK (datetime(timestamp) NOT NULL),
                                          value real NOT NULL)",
        );
        check_table(
            "transactions",
            "CREATE TABLE transactions (guid text PRIMARY KEY NOT NULL,
                                                num text NOT NULL,
                                                post_date text
                                                    CHECK (datetime(post_date) NOT NULL),
                                                enter_date text
                                                    CHECK (datetime(enter_date) NOT NULL),
                                                description text)",
        );
        check_table(
            "splits",
            "CREATE TABLE splits (guid text PRIMARY KEY NOT NULL,
                                          tx_guid text NOT NULL REFERENCES transactions (guid),
                                          account_guid text NOT NULL REFERENCES accounts (guid),
                                          memo text,
                                          flags integer,
                                          value bigint NOT NULL,
                                          quantity bigint NOT NULL)",
        );
        check_table(
            "commodities",
            "CREATE TABLE commodities (guid text PRIMARY KEY NOT NULL,
                                               mnemonic text NOT NULL,
                                               fullname text,
                                               cusip text)",
        );
        check_table(
            "scheduled_transactions",
            "CREATE TABLE scheduled_transactions
                            (guid text PRIMARY KEY NOT NULL REFERENCES transactions (guid),
                             last_used double NOT NULL)",
        );
        check_table(
            "stock_splits",
            "CREATE TABLE stock_splits (commodity_guid text NOT NULL REFERENCES \
                     commodities (guid),
                                                split_date text NOT NULL,
                                                split_factor real NOT NULL)",
        );
    }

    // Make sure the essential children of the root account are present
    // and their flags are set correctly
    {
        struct AccountInfo {
            guid: String,
            flags: i32,
        }
        let mut get_root_child_stmt = db
            .prepare(
                "select guid, flags from accounts
                where name=?1 and parent_guid=(select root_account_guid from book)",
            )
            .unwrap();
        let mut create_root_child_stmt = db
            .prepare(concat!(
                "insert into accounts (guid, name, parent_guid, code, description, flags)
                          values (",
                constants!(NEW_UUID),
                ", ?1,
                          (select root_account_guid from book), '', '', ?2)"
            ))
            .unwrap();
        let mut update_child_flags_stmt =
            db.prepare("update accounts set flags = ?1 where guid = ?2").unwrap();
        let mut process_root_child = |account_name: &str,
                                      correct_flags: i32,
                                      bits_to_ignore: i32| {
            let mut get_root_child_iter = get_root_child_stmt
                .query_map(params![account_name], |row| {
                    Ok(AccountInfo {
                        guid: row.get(0).unwrap(),
                        flags: row.get(1).unwrap(),
                    })
                })
                .unwrap();
            match get_root_child_iter.next() {
                Some(root_child) => {
                    let r = root_child.unwrap();
                    // Account exists and is hopefully unique (this will be checked
                    // immediately after this). Check flags.
                    // Make a mask by complementing the bits
                    let mask = bits_to_ignore ^ 0x7fffffff;
                    if (r.flags & mask) != (correct_flags & mask) {
                        let correction = r.flags & bits_to_ignore | correct_flags;
                        println!(
                            "The current flags of Root:{} are not correct. Current value: \
                             0x{:x}. Should be 0x{:x}. This will be fixed.",
                            account_name, r.flags, correction
                        );
                        update_child_flags_stmt.execute(params![correction, r.guid]).unwrap();
                    }
                    if get_root_child_iter.count() != 0 {
                        panic!("path_to_guid query returned more than one row")
                    };
                }
                None => {
                    // Account doesn't exist, create it
                    println!("The account Root:{} is missing and will be created.", account_name);
                    create_root_child_stmt.execute(params![account_name, correct_flags]).unwrap();
                }
            }
        };
        process_root_child(
            "Assets",
            ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS | ACCOUNT_FLAG_PLACEHOLDER | ACCOUNT_FLAG_PERMANENT,
            0,
        );
        process_root_child(
            "Liabilities",
            ACCOUNT_FLAG_DESCENDENTS_ARE_LIABILITIES
                | ACCOUNT_FLAG_PLACEHOLDER
                | ACCOUNT_FLAG_PERMANENT,
            0,
        );
        process_root_child(
            "Income",
            ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME | ACCOUNT_FLAG_PLACEHOLDER | ACCOUNT_FLAG_PERMANENT,
            0,
        );
        process_root_child(
            "Expenses",
            ACCOUNT_FLAG_DESCENDENTS_ARE_EXPENSES
                | ACCOUNT_FLAG_PLACEHOLDER
                | ACCOUNT_FLAG_PERMANENT,
            0,
        );
        process_root_child(
            "Equity",
            ACCOUNT_FLAG_NOCHILDREN | ACCOUNT_FLAG_PERMANENT | ACCOUNT_FLAG_HIDDEN,
            0,
        );
        process_root_child(
            "Unspecified",
            ACCOUNT_FLAG_HIDDEN | ACCOUNT_FLAG_PERMANENT | ACCOUNT_FLAG_NOCHILDREN,
            0,
        );
    }

    {
        let root_info = db
            .query_row(
                "
            select name, guid, flags from accounts where guid in (select root_account_guid from \
                                      book)",
                params![],
                |row| {
                    Ok(Account {
                        name: row.get(0).unwrap(),
                        path: "".to_string(),
                        guid: row.get(1).unwrap(),
                        commodity_guid: "".to_string(),
                        flags: row.get(2).unwrap(),
                    })
                },
            )
            .unwrap();

        let mut globals =
            Globals { new_guid: db.prepare(concat!("select ", constants!(NEW_UUID))).unwrap(),
                      insert_new_commodity: db.prepare(
                                                       "insert into commodities
                                                (guid,  mnemonic, fullname, cusip)
                                                values (?1, '**unknown**', ?2, '')",
            )
                                              .unwrap(),
                      link_to_commodity: db.prepare("update accounts set commodity_guid = ?1 \
                                                     where guid = ?2")
                                           .unwrap(),
                      verify_commodity_guid: db.prepare("select guid from commodities where \
                                                         guid=?1")
                                               .unwrap(),
                      nullify_commodity_guid: db.prepare("update accounts set commodity_guid \
                                                          = NULL where guid = ?1")
                                                .unwrap(),
                      check_commodity_name: db.prepare(
                                                       "select c.fullname==a.name from \
                                                        accounts a, commodities c
                                                where a.guid=?1 and c.guid=a.commodity_guid",
            )
                                              .unwrap(),
                      count_transactions: db.prepare("select count(guid) from splits where \
                                                      account_guid=?1")
                                            .unwrap(),
                      get_possible_commodity_guid: db.prepare("select ifnull(guid, '') from \
                                                               commodities where fullname = ?1")
                                                     .unwrap(),
                      find_children: db.prepare(
                                                "select name, guid, ifnull(commodity_guid, \
                                                 ''), flags
                                        from accounts where parent_guid = ?1",
            )
                                       .unwrap(),
                      check_quantities: db.prepare("select count(*) from splits where \
                                                    quantity <> 0 and account_guid = ?1")
                                          .unwrap(),
                      fix_quantities: db.prepare("update splits set quantity = 0 where \
                                                  quantity <> 0 and account_guid = ?1")
                                        .unwrap(),
                      check_money_market: db.prepare(concat!("select ifnull(c.flags & ", constants!(COMMODITY_FLAG_MONEY_MARKET_FUND), ", 0)
                                                        from commodities c, accounts a where a.guid=?1 and c.guid=a.commodity_guid"))
                                        .unwrap(),
                      fix_money_market_quantities: db.prepare("update splits set quantity = value where quantity <> value and account_guid = ?1")
                                        .unwrap() };

        globals.walk_account_tree(&root_info, 0);
    }

    // Delete any splits that point to non-existent transactions
    let no_transaction_count: i32 = db
        .query_row(
            "select count(*) from splits s left outer join transactions t on s.tx_guid \
                      = t.guid
                    where t.guid is null",
            params![],
            get_result!(i32),
        )
        .unwrap();
    if no_transaction_count > 0 {
        println!(
            "There were {} splits pointing to non-existent transactions. They will be \
             deleted.",
            no_transaction_count
        );
        db.execute(
            "delete from splits where guid in
                        (select s.guid
                         from splits s left outer join transactions t on s.tx_guid = t.guid
                         where t.guid is null)",
            params![],
        )
        .unwrap();
    };

    // List transactions that have splits that point to non-existent accounts
    {
        let mut first_p = true;
        let mut stmt = db
            .prepare(
                "
            select date(post_date), description
            from transactions
            where guid in (select s.tx_guid
                            from splits s left outer join accounts a on s.account_guid = a.guid
                            where a.guid is null)
            order by post_date, description
            ",
            )
            .unwrap();
        let no_account_iter = stmt.query_map(params![], get_result!(string_string)).unwrap();
        for result in no_account_iter {
            let (date, description) = result.unwrap();
            if first_p {
                println!(
                    "The following transactions have splits pointing to a non-existent \
                     account and need to be repaired:"
                );
            }
            first_p = false;
            println!("{}, {}", date, description);
        }
    }
    // Delete any transactions that have no splits
    let no_split_count: i32 = db
        .query_row(
            "select count(*) from transactions
                    where guid not in (select tx_guid from splits)",
            params![],
            get_result!(i32),
        )
        .unwrap();
    if no_split_count > 0 {
        println!(
            "There were {} transactions that have no splits. They will be deleted.",
            no_split_count
        );
        db.execute(
            "delete from transactions where guid in
                        (select guid from transactions
                         where guid not in (select tx_guid from splits))",
            params![],
        )
        .unwrap();
    };

    // Check all transactions to be sure they are balanced
    {
        struct NotBalanced {
            post_date: String,
            description: String,
            balance: f64,
            transaction_guid: String,
        }
        const BALANCE_CHECK_SQL: &str = "
            select t.post_date, t.description, s.total, s.tx_guid
            from ( select tx_guid, sum(value) as total
                   from splits group by tx_guid) s, transactions t
            where abs(s.total) > .0001 and t.guid=s.tx_guid
            order by t.post_date";
        const SPLIT_ACCOUNTS_SQL: &str = "
            select account_guid from splits where tx_guid=?1";
        let mut first_p: bool = true;
        let mut balance_check_stmt = db.prepare(BALANCE_CHECK_SQL).unwrap();
        let mut split_accounts_stmt = db.prepare(SPLIT_ACCOUNTS_SQL).unwrap();
        let mut guid_to_path_stmt = db.prepare(GUID_TO_PATH_SQL).unwrap();
        let balance_check_iter = balance_check_stmt
            .query_map(params![], |row| {
                Ok(NotBalanced {
                    post_date: row.get(0).unwrap(),
                    description: row.get(1).unwrap(),
                    balance: row.get(2).unwrap(),
                    transaction_guid: row.get(3).unwrap(),
                })
            })
            .unwrap();
        for transaction in balance_check_iter {
            let t = transaction.unwrap();
            if first_p {
                println!("The following transactions are not balanced:");
                first_p = false;
            }
            println!("{} {} {}", t.post_date, t.description, t.balance);
            let split_accounts_iter = split_accounts_stmt
                .query_map(params![t.transaction_guid], get_result!(string))
                .unwrap();
            for account_guid in split_accounts_iter {
                let ag = account_guid.unwrap();
                println!("\t{}", guid_to_path(&mut guid_to_path_stmt, &ag));
            }
        }
    }

    // Check for duplicate symbols
    {
        struct DuplicateCommodities {
            mnemonic: String,
            fullname: String,
            cusip: String,
            guid: String,
        }
        const DUPLICATE_SYMBOL_CHECK_SQL: &str = "
            select ifnull(mnemonic, ''), fullname, ifnull(cusip,''), guid
            from commodities
            where mnemonic <> '' and
                mnemonic in (select mnemonic
                                from (select count(*) as n, mnemonic from commodities
                                      group by mnemonic)
                                where n>1)
            order by mnemonic";
        let mut first_p: bool = true;
        // Check commodities to be sure that the symbols are unique
        let mut duplicate_symbol_check_stmt = db.prepare(DUPLICATE_SYMBOL_CHECK_SQL).unwrap();
        let duplicate_symbol_check_iter = duplicate_symbol_check_stmt
            .query_map(params![], |row| {
                Ok(DuplicateCommodities {
                    mnemonic: row.get(0).unwrap(),
                    fullname: row.get(1).unwrap(),
                    cusip: row.get(2).unwrap(),
                    guid: row.get(3).unwrap(),
                })
            })
            .unwrap();
        for commodity in duplicate_symbol_check_iter {
            let c = commodity.unwrap();
            if first_p {
                println!("Duplicated symbols:");
                first_p = false;
            }
            println!("{}|{}|{}|{}", c.mnemonic, c.fullname, c.cusip, c.guid);
        }
    }

    // Look for orphaned accounts -- accounts with no parent that are not the root account
    {
        let mut orphans_stmt = db
            .prepare(
                "select name, guid
                                            from accounts
                                            where parent_guid is null and guid not in (select \
                                           root_account_guid from book)",
            )
            .unwrap();
        let orphans_iter = orphans_stmt.query_map(params![], get_result!(string_string)).unwrap();
        for wrapped_orphan in orphans_iter {
            let (name, guid) = wrapped_orphan.unwrap();
            println!(
                "Orphaned account {} found. Temporarily making Root its parent. Please use \
                 Newcash re-parenting to place correctly in the account tree.",
                name
            );
            // Note that the guid is appended to the name, to avoid duplicate name errors.
            let mut stmt = db
                .prepare_cached(
                    "update accounts set name = name||'.'||guid, \
                     parent_guid = (select root_account_guid from book) \
                     where guid=?1",
                )
                .unwrap();
            stmt.execute(params![guid]).unwrap();
        }
    }

    db.execute("vacuum", params![]).unwrap();
}
