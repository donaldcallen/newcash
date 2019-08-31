extern crate rusqlite;

use rusqlite::{params, Connection, Statement};

#[macro_export]
macro_rules! constants {
    (ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS) => {
        "(1<<2)"
    };
    (ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME) => {
        "(1<<6)"
    };
    (ACCOUNT_FLAG_HIDDEN) => {
        "(1<<1)"
    };
    (EPSILON) => {
        ".01"
    };
    (NEW_UUID) => {
        "lower(hex(randomblob(16)))"
    };
    (SPLIT_FLAG_RECONCILED) => {
        "(1<<0)"
    };
    (SPLIT_FLAG_TRANSFER) => {
        "(1<<1)"
    };
    (COMMODITY_FLAG_MONEY_MARKET_FUND) => {
        "(1<<0)"
    };
}

#[macro_export]
macro_rules! cache_statement_locally {
    ($sql:expr) => {
        unsafe {
            static mut STATEMENT: Option<Statement<'static>> = None;
            if STATEMENT.is_none() {
                STATEMENT = Some((&DB).prepare($sql).unwrap());
            };
            STATEMENT.as_mut().unwrap()
        }
    };
}

#[macro_export]
macro_rules! cache_statement_globally {
    ($sql:expr, $stmt:expr) => {
        unsafe {
            if $stmt.is_none() {
                $stmt = Some((&DB).prepare($sql).unwrap());
            };
            $stmt.as_mut().unwrap()
        }
    };
}

#[macro_export]
macro_rules! get_result {
    (string) => {
        |row| -> Result<String, rusqlite::Error> { Ok(row.get(0).unwrap()) }
    };
    (i32) => {
        |row| -> Result<i32, rusqlite::Error> { Ok(row.get(0).unwrap()) }
    };
    (f64) => {
        |row| -> Result<f64, rusqlite::Error> { Ok(row.get(0).unwrap()) }
    };
    (f64_f64) => {
        |row| -> Result<(f64, f64), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap()))
        }
    };
    (bool_bool) => {
        |row| -> Result<(bool, bool), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap()))
        }
    };
    (string_string) => {
        |row| -> Result<(String, String), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap()))
        }
    };
    (string_string_string) => {
        |row| -> Result<(String, String, String), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap(), row.get(2).unwrap()))
        }
    };
    (string_i32) => {
        |row| -> Result<(String, i32), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap()))
        }
    };
    (string_string_i32) => {
        |row| -> Result<(String, String, i32), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap(), row.get(2).unwrap()))
        }
    };
    (string_string_f64) => {
        |row| -> Result<(String, String, f64), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap(), row.get(2).unwrap()))
        }
    };
    (string_string_string_i32) => {
        |row| -> Result<(String, String, String, i32), rusqlite::Error> {
            Ok((row.get(0).unwrap(), row.get(1).unwrap(), row.get(2).unwrap(), row.get(3).unwrap()))
        }
    };
}

pub mod constants;
pub mod queries;

// Functions
pub fn path_to_guid(db: &Connection, account_path: &str) -> String {
    let namelist = account_path.rsplit(':');
    let mut count = 1;
    let mut from_clause: String = String::from("");
    let mut where_clause: String = String::from("");
    let length = namelist.clone().count() - 1;
    for name in namelist {
        from_clause.push_str(&format!(", accounts a{}", count));
        if count == length {
            where_clause.push_str(&format!(" and a{}.name='{}' and a{}.parent_guid=(select \
                                            root_account_guid from book)",
                                           count, name, count));
            break;
        } else {
            let new_count = count + 1;
            where_clause.push_str(&format!(" and a{}.name='{}' and a{}.parent_guid=a{}.guid",
                                           count, name, count, new_count));
            count = new_count;
        }
    }

    // Trim the results
    let final_from_clause = from_clause.trim_start_matches(',');
    let (_, final_where_clause) = where_clause.split_at(5);
    // Assemble and execute the query
    db.query_row(format!("select a1.guid from{} where {}",
                         final_from_clause, final_where_clause).as_str(),
                 params![],
                 |row| {
                     Ok({
                         let result: String = row.get(0).unwrap();
                         result
                     })
                 })
      .unwrap()
}

// Takes GUID_TO_PATH_SQL prepared
pub fn guid_to_path(stmt: &mut Statement, account_guid: &str) -> String {
    let mut current_guid: String = account_guid.to_string();
    let mut path: String = "".to_string();
    struct Account {
        name: String,
        guid: String,
    }

    loop {
        let maybe_account = stmt.query_row(&[&current_guid], |row| {
                                    Ok({
                                        Account { name: row.get(0).unwrap(),
                                                  guid: row.get(1).unwrap() }
                                    })
                                });
        match maybe_account {
            Ok(a) => {
                path = format!("{}:{}", a.name, path);
                current_guid = a.guid;
            }
            Err(_) => return format!(":{}", path.trim_end_matches(':')),
        }
    }
}

// Takes INHERITED_P_SQL prepared
pub fn inherited_p(stmt: &mut Statement, account_guid: &str, flag_bit: i32) -> bool {
    struct Account {
        guid: String,
        flags: i32,
    }
    let mut child_guid: String = account_guid.to_string();

    // Here a1 is the parent account, a2 is the account we are starting from
    loop {
        let maybe_account = stmt.query_row(&[&child_guid], |row| {
                                    Ok({
                                        Account { guid: row.get(0).unwrap(),
                                                  flags: row.get(1).unwrap() }
                                    })
                                });
        match maybe_account {
            Ok(p) => {
                if (p.flags & flag_bit) != 0 {
                    return true;
                } else {
                    // If we get here, we haven't reached the root yet or found an ancestor with the flag bit set
                    child_guid = p.guid;
                }
            }
            Err(_) => return false,
        }
    }
}
