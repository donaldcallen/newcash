m4_include(`../newcash.m4')m4_dnl
#!/usr/bin/env tclsh

package require sqlite3

## Constants
m4_define(BookNameIndex,0)m4_dnl
m4_define(DbFileIndex,1)m4_dnl
m4_define(CommandLineArgs,`bookName pathToDatabase')m4_dnl

## Procedures
proc oneRowOneColumn {sql} {
    set result [db eval $sql]
    switch [llength $result] {
        0 {
            puts "The query $sql failed to return a row"
        exit 1
        }
        1 {
            return $result
        }
        2 {
            puts "The query $sql returned more than one row"
        exit 1
        }
    }
}

## Check that the number of arguments is correct
if {$argc != [llength CommandLineArgs]} {
    puts "Usage: newcashConvertGnucashDatabase CommandLineArgs"
    exit 1
}

## Get the arguments
set dbFile [lindex $argv DbFileIndex]
set bookName [lindex $argv BookNameIndex]

## Open the database
sqlite3 db $dbFile

set rootAccountGuid [oneRowOneColumn {select root_account_guid from books}]

# Create new accounts table, copy appropriate fields, delete old table and rename new one 
# Set tax-related flag bit where appropriate
# Drop old accounts table and rename new one
db eval {
create table new_accounts (
      guid text PRIMARY KEY NOT NULL,
      name text NOT NULL,
      parent_guid text REFERENCES accounts (guid),
      commodity_guid text REFERENCES commodities (guid),
      code text,
      description text,
      flags integer);
insert into new_accounts (
      guid,
      name,
      parent_guid,
      commodity_guid,
      code,
      description,
      flags)
     select 
       guid,
       name,
       parent_guid,
       commodity_guid,
       code,
       description,
       (hidden*AccountFlagHiddenBit)|(placeholder*AccountFlagPlaceHolderBit) from accounts;
update new_accounts set flags = flags | AccountFlagSelfAndDescendentsAreTaxRelatedBit
    where guid in (select obj_guid from slots where name='tax-related');
drop table accounts;
alter table new_accounts rename to accounts}

# Create new transactions table, copy appropriate fields, delete old table and rename new one 
# Drop old transactions table and rename new one
db eval {
CREATE TABLE new_transactions (
	guid text PRIMARY KEY NOT NULL,
	num text NOT NULL,
	post_date text CHECK (datetime(post_date) NOT NULL),
	enter_date text CHECK (datetime(enter_date) NOT NULL),
	description text);
insert into new_transactions (
      guid,
      num,
      post_date,
      enter_date,
      description)
     select 
       guid,
      num,
      post_date,
      enter_date,
      description from transactions;
drop table transactions;
alter table new_transactions rename to transactions}

# Insure that the Assets, Liabilites, Income, Expenses, and Equity accounts exist and set their flag bits
foreach name {Assets Liabilities Income Expenses Equity Unspecified} \
    flags [list [expr AccountFlagDescendentsAreAssetsBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit] \
                [expr AccountFlagDescendentsAreLiabilitiesBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit] \
                [expr AccountFlagDescendentsAreIncomeBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit] \
                [expr AccountFlagDescendentsAreExpensesBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit] \
                [expr AccountFlagNoChildrenBit|AccountFlagHiddenBit|AccountFlagPermanentBit] \
                [expr AccountFlagNoChildrenBit|AccountFlagHiddenBit|AccountFlagPermanentBit]] {
                set rootChildGuid  [db eval {select guid from accounts where name = $name and parent_guid = $rootAccountGuid}]
                switch [llength $rootChildGuid] {
                    0 {                                        
                        # The child doesn't exist. Create it, setting the appropriate flag bit, and tell the user.
                        db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
                            values (NewUUID, $name, $rootAccountGuid, '', '', $flags)}
                        puts "An account named Root:$name was not found. The account has been created."
                    }
                    1 {
                        db eval {update accounts set flags = flags | $flags where guid = $rootChildGuid}
                    }
                    default {
                        puts "The query to obtain the guid of the Root:$name account returned more than one row. Sibling accounts should have unique names. You must fix this with Gnucash."
                        exit 1
                    }
                }
            }

# Fix date formats in transactions and prices tables and limit the length of the num field in transactions
db eval {
update transactions set post_date =
    substr (post_date,1,4)||'-'||substr (post_date,5,2)||'-'||substr (post_date,7,2)||' '||substr (post_date,9,2)||':'||substr (post_date,11,2)||':'||substr (post_date,13,2);
update transactions set enter_date =
    substr (enter_date,1,4)||'-'||substr (enter_date,5,2)||'-'||substr (enter_date,7,2)||' '||substr (enter_date,9,2)||':'||substr (enter_date,11,2)||':'||substr (enter_date,13,2);
update prices set date = substr (date,1,4)||'-'||substr (date,5,2)||'-'||substr (date,7,2)||' '||substr (date,9,2)||':'||substr (date,11,2)||':'||substr (date,13,2);
update transactions set num = substr (num,1,9) where length(num)>9}

# Create book table and drop the old books table
db eval {
    create table book (
        root_account_guid text NOT NULL REFERENCES accounts (guid),
        name text);
    insert into book (root_account_guid, name)
        values ($rootAccountGuid, $bookName);
    drop table books}

# Create new commodities table
# Copy appropriate data from Gnucash commodities table, then drop the old table and rename the new one
db eval {
    create table new_commodities (
        guid text PRIMARY KEY NOT NULL,
        mnemonic text NOT NULL,
        fullname text,
        cusip text);
    insert into new_commodities (guid, mnemonic, fullname, cusip)
        select guid, mnemonic, fullname, cusip from commodities;
    drop table commodities;
    alter table new_commodities rename to commodities}

# Create new prices table
# Copy appropriate data from Gnucash prices table, then drop the old table and rename the new one
db eval {
    create table new_prices (
        guid text PRIMARY KEY NOT NULL,
        commodity_guid text NOT NULL REFERENCES commodities (guid),
        timestamp text NOT NULL CHECK (datetime(timestamp) NOT NULL),
        value real NOT NULL);
    insert into new_prices (guid, commodity_guid, timestamp, value)
        select guid, commodity_guid, date, (cast (value_num as double))/(cast (value_denom as double)) from prices;
    drop table prices;
    alter table new_prices rename to prices}

# Create new splits table, copy appropriate data, drop the old table and rename the new one        
db eval {
    create table new_splits (
        guid text PRIMARY KEY NOT NULL,
        tx_guid text NOT NULL REFERENCES transactions (guid),
        account_guid text NOT NULL REFERENCES accounts (guid),
        memo text,
        flags integer,
        value bigint NOT NULL,
        quantity bigint NOT NULL);
    insert into new_splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
        select guid,
            tx_guid,
            account_guid,
            memo,
            ((action='Transfer')*SplitFlagTransferBit)|((reconcile_state='y')*SplitFlagReconciledBit),
            value_num*MaximumDenominator/value_denom,
            quantity_num*MaximumDenominator/quantity_denom
        from splits;
    drop table splits;
    alter table new_splits rename to splits}

# Get rid of tables we don't need        
db eval {
    drop table gnclock;
    drop table versions;
    drop table budgets;
    drop table budget_amounts;
    drop table lots;
    drop table schedxactions;
    drop table billterms;
    drop table employees;
    drop table invoices;
    drop table orders;
    drop table taxtable_entries;
    drop table recurrences;
    drop table customers;
    drop table jobs;
    drop table taxtables;
    drop table vendors;
    drop table entries;
    drop table slots}

# Delete unnecessary accounts
db eval {delete from accounts where (name='Imbalance-USD' or name='Orphan-USD') and parent_guid=$rootAccountGuid}

# More housekeeping
db eval {
    create table scheduled_transactions (
            guid text PRIMARY KEY NOT NULL REFERENCES transactions (guid),
            last_used double NOT NULL
            );
    create table stock_splits (
	guid text PRIMARY KEY NOT NULL,
	commodity_guid text NOT NULL REFERENCES commodities (guid),
    split_date text NOT NULL,
    split_factor real NOT NULL);
    create unique index unique_accounts on accounts (parent_guid, name);
    create index commodities_index on commodities (guid);
    create index parents on accounts (parent_guid);
    create index price_by_commodity on prices (commodity_guid);
    create index splits_tx_guid_index on splits (tx_guid);
    create index splits_account_guid_index on splits (account_guid);
    vacuum}

