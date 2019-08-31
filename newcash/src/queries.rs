// Copyright (C) 2018 Donald C. Allen
//
// This file is part of the Newcash Personal Finance Suite.
//
// Newcash is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// The Newcash Suite is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You have received a copy of the GNU General Public License
// along with the Newcash Suite.  It is also available at <http://www.gnu.org/licenses/>.

pub const ACCOUNT_CHILD_ALL_SQL: &str = "
    select name, guid, flags
    from accounts
    where parent_guid = ?1
    order by name";
pub const ACCOUNT_CHILD_NOT_HIDDEN_SQL: &str = concat!(
                                                       "
    select name, guid, flags
    from accounts
    where parent_guid = ?1 and not (flags&",
                                                       constants!(ACCOUNT_FLAG_HIDDEN),
                                                       ")
    order by name"
);
pub const ACCOUNT_INFORMATION_SQL: &str = "
    select ifnull(a.name, ''), ifnull(a.code, ''), ifnull(a.description,''), ifnull(a.flags, 0) 
    from accounts a 
    where a.guid = ?1";
pub const NEW_ACCOUNT_WITH_COMMODITY_SQL: &str = "
    insert into accounts (guid, name, parent_guid, code, description, flags, commodity_guid) 
                        values(?1, ?2, ?3, ?4, ?5, ?6, ?7)";
pub const NEW_ACCOUNT_WITHOUT_COMMODITY_SQL: &str = "
    insert into accounts (guid, name, parent_guid, code, description, flags, commodity_guid) 
                        values(?1, ?2, ?3, ?4, ?5, ?6, NULL)";
pub const UPDATE_ACCOUNT_WITH_COMMODITY_SQL: &str = "
    update accounts set name=?1, code=?2, description=?3, flags=?4, commodity_guid=?5 where guid = \
                                                     ?6";
pub const UPDATE_ACCOUNT_WITHOUT_COMMODITY_SQL: &str = "
    update accounts set name=?1, code=?2, description=?3, flags=?4, commodity_guid=NULL where guid \
                                                        = ?5";
pub const BASIC_INFO_SQL: &str = "
    select b.root_account_guid, b.name, a.guid
    from book b, accounts a
    where a.parent_guid=b.root_account_guid and a.name='Unspecified'";
pub const UNBALANCED_TRANSACTIONS_SQL: &str = "select sum(value) from splits where tx_guid=?1";
pub const COMMODITY_INFO_SQL: &str =
    "select guid, fullname from commodities where fullname like ?1 order by fullname";
pub const GET_COMMODITY_GUID_SQL: &str =
    "select ifnull(commodity_guid, '') from accounts where guid=?1";
pub const DUPLICATE_CHECK_SQL: &str =
    "select guid from accounts where name = ?1 and parent_guid = ?2";
pub const DELETE_ACCOUNT_SPLIT_CHECK_STR: &str = "
    select s.nsplits+c.nchildren
    from 
    (select count(*) as nsplits from splits where account_guid = ?1) s, 
    (select count(*) as nchildren from accounts where parent_guid = ?1) c";
pub const DELETE_ACCOUNT_SQL: &str = "delete from accounts where guid = ?1";
pub const PASTE_ACCOUNT_SQL: &str = concat!(
                                            "
    insert into accounts (guid, name, parent_guid, commodity_guid, code, description, flags)
    select ",
                                            constants!(NEW_UUID),
                                            ", name, ?2, commodity_guid, code, description, \
                                             flags from accounts where guid = ?1"
);
pub const MARKETABLE_ACCOUNT_REGISTER_SQL: &str = "
    select date(t.post_date)
        , ifnull(num, '')
        , ifnull(description, '')
        ,s.flags
        ,t.guid
        ,s.value
        ,s.quantity
        ,s.guid
    from transactions t, splits s 
    where s.account_guid = ?1
       and s.tx_guid = t.guid 
       and ((s.quantity != 0.0) or ((s.quantity = 0.0) and 
           (select count(*) from splits s where s.tx_guid = t.guid and s.account_guid = ?1) = 1)) 
    order by post_date, enter_date";
pub const NON_MARKETABLE_ACCOUNT_REGISTER_SQL: &str = "
    select date(post_date) 
            , ifnull(num, '')
            , ifnull(description, '')
            , flags 
            , guid 
            , value 
    from ( select datetime(max(t.post_date)) as post_date 
           , datetime(max(t.enter_date)) as enter_date 
           , max(t.num) as num 
           , max(t.description) as description 
           , max(s.flags) as flags 
           , sum(s.value) as value 
           , t.guid as guid 
           from transactions t, splits s 
           where s.account_guid = ?1
            and s.tx_guid = t.guid 
           group by t.guid 
         ) 
    order by post_date, enter_date";
pub const GET_SPLIT_FACTOR_SQL: &str = "
    select ifnull(exp(sum(log(ss.split_factor))), 1.0) 
    from stock_splits ss, accounts a, splits s, transactions t 
    where s.guid = ?1 
        and a.guid = s.account_guid 
        and t.guid = s.tx_guid 
        and ss.commodity_guid = a.commodity_guid 
        and ss.split_date > date(t.post_date)";
pub const STOCK_SPLITS_REGISTER_SQL: &str = "
    select ss.guid, date(ss.split_date), ss.split_factor 
    from stock_splits ss 
    where ss.commodity_guid = ?1 
    order by ss.split_date";
pub const STOCK_SPLIT_INCREMENT_DATE_SQL: &str =
    "update stock_splits set split_date = date(split_date, ?1 || ' days') where guid = ?2";
pub const STOCK_SPLIT_DATE_TO_FIRST_OF_MONTH_SQL: &str =
    "update stock_splits set split_date = datetime(split_date, 'start of month') where guid = ?1";
pub const STOCK_SPLIT_DATE_TO_END_OF_MONTH_SQL: &str =
    "update stock_splits set split_date = datetime(split_date, 'start of month', '31 days', \
     'start of month', '-1 days') where guid = ?1";
pub const STOCK_SPLIT_DATE_TO_USER_ENTRY_SQL: &str =
    "update stock_splits set split_date = date(?1) where guid = ?2";
pub const STOCK_SPLIT_DATE_TODAY_SQL: &str = "
    update stock_splits set split_date = date('now', 'localtime') where guid = ?1";
pub const INCREMENT_TRANSACTION_DATE_SQL: &str = "
    update transactions set post_date = datetime(post_date, ?1 || ' days') where guid = ?2";
pub const TRANSACTION_DATE_TO_FIRST_OF_MONTH_SQL: &str = "
    update transactions set post_date = datetime(post_date, 'start of month') where guid = ?1";
pub const TRANSACTION_DATE_TO_END_OF_MONTH_SQL: &str = "
    update transactions set post_date = datetime(post_date, 'start of month', '31 days', 'start of \
                                                        month', '-1 days')
    where guid = ?1";
pub const TRANSACTION_DATE_TO_USER_ENTRY_SQL: &str = "
    update transactions set post_date = datetime(?1||'12:00:00', 'localtime') where guid = ?2";
pub const TRANSACTION_DATE_TODAY_SQL: &str = "
    update transactions set post_date = datetime('now', 'localtime') where guid = ?1";
pub const UPDATE_SPLIT_FACTOR_SQL: &str =
    "update stock_splits set split_factor = ?1 where guid = ?2";
pub const NEW_STOCK_SPLIT_SQL: &str =
    concat!("insert into stock_splits (guid, commodity_guid, split_date, split_factor)
                            values (",
            constants!(NEW_UUID),
            ", ?1, date('now', 'localtime'), 1.0)");
pub const DELETE_STOCK_SPLIT_SQL: &str = "delete from stock_splits where guid = ?1";
pub const TOGGLE_TRANSACTION_R_FLAG_SQL: &str = concat!(
                                                        "
    update splits set flags = ((flags & ~",
                                                        constants!(SPLIT_FLAG_RECONCILED),
                                                        ")|(~flags & ",
                                                        constants!(SPLIT_FLAG_RECONCILED),
                                                        "))
    where tx_guid = ?1 and account_guid = ?2"
);
pub const REPARENT_ACCOUNT_SQL: &str = "update accounts set parent_guid = ?1 where guid = ?2";
pub const NEW_TRANSACTION_SQL: &str = "
    insert into transactions (guid, num, post_date, enter_date, description) 
        select ?1, '', 
            (select ifnull(datetime(max(julianday(t.post_date))),datetime('now', 'localtime')) 
            from transactions t, splits s 
            where s.account_guid = ?2 and t.guid = s.tx_guid), 
        datetime('now', 'localtime'), ''";
pub const NEW_TRANSACTION_SPLIT_SQL: &str = concat!(
                                                    "
    insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
                       select ",
                                                    constants!(NEW_UUID),
                                                    ", ?1, ?2, '', 0, 0, 0"
);
pub const DUPLICATE_TRANSACTION_NO_DATE_SQL: &str = "
    insert into transactions (guid, num, post_date, enter_date, description) 
                        select ?1 
                        , '' 
                        , datetime('now', 'localtime') 
                        , datetime('now', 'localtime') 
                        , (select description from transactions where guid = ?2)";
pub const DUPLICATE_TRANSACTION_WITH_DATE_SQL: &str = "
    insert into transactions (guid, num, post_date, enter_date, description) 
                            select ?1 
                            , '' 
                            , datetime(?3||' 12:00:00', 'localtime') 
                            , datetime('now', 'localtime') 
                            , (select description from transactions where guid = ?2)";
pub const DUPLICATE_TRANSACTION_SPLITS_SQL: &str = concat!(
                                                           "
    insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
                        select ",
                                                           constants!(NEW_UUID),
                                                           ", ?1, account_guid, memo, 0, value, \
                                                            quantity from splits
                        where tx_guid = ?2"
);
pub const DELETE_TRANSACTION_SPLITS_SQL: &str = "delete from splits where tx_guid = ?1";
pub const DELETE_TRANSACTION_SQL: &str = "delete from transactions where guid = ?1";
pub const UPDATE_MEMO_SQL: &str = "update splits set memo = ?1 where guid = ?2";
pub const MONEY_MARKET_P_SQL: &str = "
    select ifnull(c.flags, 0)
    from commodities c, accounts a, splits s
    where a.guid=s.account_guid
        and c.guid=a.commodity_guid
        and s.guid=?1";
pub const PRICE_EDITED_NULL_CHECK_SQL: &str = "select ((quantity is null) or (quantity == \
                                               0)),((value is null) or (value == 0)) from splits \
                                               where guid=?1";
pub const SPLIT_COUNT_SQL: &str = "select count(*) from splits where tx_guid = ?1";
pub const GET_BALANCING_SPLIT_GUIDS_SQL: &str =
    "select guid, account_guid from splits where tx_guid = ?1 and guid != ?2";
pub const UPDATE_BALANCING_SPLIT_VALUE_SQL: &str = "update splits set value = (select -value from \
                                                    splits where guid = ?2) where tx_guid = ?1 \
                                                    and guid != ?2";
pub const UPDATE_BALANCING_MONEY_MARKET_SPLIT_SQL: &str =
    "update splits set (value, quantity) = (select -value, -value from splits where guid = ?2) \
     where tx_guid = ?1 and guid != ?2";
pub const CHECK_TRANSACTION_BALANCE_SQL: &str = "select sum(value) from splits where tx_guid=?1";
pub const MARKETABLE_TRANSACTION_REGISTER_SQL: &str = "
    select s.account_guid, s.guid, s.memo, s.flags, ifnull(s.value, 0), ifnull(s.quantity, 0) 
    from splits s 
    where s.tx_guid=?1 
    order by s.memo";
pub const NON_MARKETABLE_TRANSACTION_REGISTER_SQL: &str = "
    select s.account_guid, s.guid, s.memo, s.flags, ifnull(s.value, 0)
    from splits s 
    where s.tx_guid = ?1 
    order by s.memo";
pub const TOGGLE_SPLIT_R_FLAG_SQL: &str = concat!("update splits set flags = ((flags & ~",
                                                  constants!(SPLIT_FLAG_RECONCILED),
                                                  ")|(~flags & ",
                                                  constants!(SPLIT_FLAG_RECONCILED),
                                                  ")) where guid = ?1");
pub const TOGGLE_SPLIT_T_FLAG_SQL: &str = concat!("update splits set flags = ((flags & ~",
                                                  constants!(SPLIT_FLAG_TRANSFER),
                                                  ")|(~flags & ",
                                                  constants!(SPLIT_FLAG_TRANSFER),
                                                  ")) where guid = ?1");
pub const NEW_SPLIT_SQL: &str = "
    insert into splits (guid, tx_guid, account_guid, memo, flags, 
                       value, quantity) 
                       values ( ?1, ?2, ?3, '', 0, 0, 0)";
pub const DUPLICATE_SPLIT_SQL: &str = "
    insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) select ?1, \
                                       tx_guid, account_guid, memo, 0, value, quantity from \
                                       splits where guid = ?2";
pub const DELETE_SPLIT_SQL: &str = "delete from splits where guid = ?1";
pub const BALANCE_TRANSACTION_SQL: &str = "update splits set value = (select -sum(value) from \
                                           splits where tx_guid=?1 and guid !=?2) where guid = ?2";
pub const REVERSE_SIGN_SQL: &str = "update splits set value = -value where guid = ?1";
pub const PASTE_ACCOUNT_GUID_SQL: &str = "update splits set account_guid = ?1 where guid = ?2";
pub const TOGGLE_COMMODITY_MM_FLAG_SQL: &str = concat!(
    "
    update commodities set flags =
        ((flags & ~",
    constants!(COMMODITY_FLAG_MONEY_MARKET_FUND),
    ")|(~flags & ",
    constants!(COMMODITY_FLAG_MONEY_MARKET_FUND),
    "))
            where guid = ?1"
);
pub const COMMODITIES_SQL: &str = "
    select guid, ifnull(mnemonic, ''), ifnull(fullname, ''), ifnull(cusip, ''), ifnull(flags, 0)
    from commodities
    order by fullname";
pub const NEW_COMMODITY_SQL: &str = "
    insert into commodities (guid, mnemonic, fullname, cusip)
                            values (?1, '', '', '')";
pub const DUPLICATE_COMMODITY_SQL: &str = "
    insert into commodities (guid, mnemonic, fullname, cusip)
                select ?1, mnemonic, fullname||' (copy)', cusip
                from commodities where guid = ?2";
pub const CHECK_INUSE_COMMODITY_SQL: &str =
    "select count(*) from accounts where commodity_guid = ?1";
pub const DELETE_COMMODITY_SQL: &str = "delete from commodities where guid = ?1";
pub const LATEST_QUOTE_TIMESTAMP_SQL: &str = "select max(timestamp) from prices";
pub const DELETE_QUOTE_SQL: &str = "delete from prices where guid = ?1";
pub const NEW_QUOTE_SQL: &str =
    concat!("insert into prices (guid, commodity_guid, timestamp, value) 
                            values (",
            constants!(NEW_UUID),
            ", ?1, datetime('now', 'localtime'), 0)");
pub const QUOTE_INCREMENT_TIMESTAMP_SQL: &str =
    "update prices set timestamp = date(timestamp, ?1 || ' days') where guid = ?2";
pub const QUOTE_TIMESTAMP_TO_FIRST_OF_MONTH_SQL: &str =
    "update prices set timestamp = datetime(timestamp, 'start of month') where guid = ?1";
pub const QUOTE_TIMESTAMP_TO_END_OF_MONTH_SQL: &str =
    "update prices set timestamp = datetime(timestamp, 'start of month', '31 days', 'start of \
     month', '-1 days') where guid = ?1";
pub const QUOTE_TIMESTAMP_TO_USER_ENTRY_SQL: &str =
    "update prices set timestamp = ?1||' 16:00:00' where guid = ?2";
pub const QUOTE_TIMESTAMP_TODAY_SQL: &str = "
    update prices set timestamp = datetime('now', 'localtime') where guid = ?1";
pub const QUOTE_UPDATE_VALUE_SQL: &str = "update prices set value = ?1 where guid = ?2";
pub const PRICES_SQL: &str = "
    select p.guid, p.timestamp, p.value
    from prices p
    where p.commodity_guid = ?1
    order by p.timestamp desc";
pub const RECONCILED_BALANCE_SQL: &str = concat!(
                                                 "
    select sum(value) 
    from splits
    where account_guid = ?1 and (flags & ",
                                                 constants!(SPLIT_FLAG_RECONCILED),
                                                 ")"
);
pub const UPDATE_MONEY_MARKET_VALUE_QUANTITY_SQL: &str =
    "update splits set value = ?1, quantity = ?1 where guid = ?2";
pub const UPDATE_VALUE_SQL: &str = "update splits set value = ?1 where guid = ?2";
pub const UPDATE_QUANTITY_SQL: &str = "update splits set quantity=?1 where guid = ?2";
pub const ACCOUNTS_LINKED_TO_COMMODITY_SQL: &str = "
    select a.guid
    from accounts a, commodities c
    where a.commodity_guid = c.guid and c.guid=?1";
