m4_include(`newcash.m4')m4_dnl
#!/usr/bin/env tclsh

package require sqlite3

## Constants
m4_define(BookNameIndex,0)m4_dnl
m4_define(DbFileIndex,1)m4_dnl
m4_define(CommandLineArgs,{bookName pathToDatabase})m4_dnl

## Check that the number of arguments is correct
if {$argc != [llength CommandLineArgs]} {
    puts "Usage: newcashCreateDatabase CommandLineArgs"
    exit 1
}

## Get the arguments
set bookName [lindex $argv BookNameIndex]
set dbFile [lindex $argv DbFileIndex]

## Open the database
sqlite3 db $dbFile

db eval {CREATE TABLE accounts (
	guid text PRIMARY KEY NOT NULL,
	name text NOT NULL,
	parent_guid text REFERENCES accounts (guid),
    commodity_guid text REFERENCES commodities (guid),
	code text,
	description text,
	flags integer)}

db eval {CREATE TABLE book (
	root_account_guid text NOT NULL REFERENCES accounts (guid),
	name text)}
db eval {CREATE TABLE prices (
	guid text PRIMARY KEY NOT NULL,
	commodity_guid text NOT NULL REFERENCES commodities (guid),
	timestamp text NOT NULL CHECK (datetime(timestamp) NOT NULL),
	value real NOT NULL)}
db eval {CREATE TABLE transactions (
	guid text PRIMARY KEY NOT NULL,
	num text NOT NULL,
	post_date text CHECK (datetime(post_date) NOT NULL),
	enter_date text CHECK (datetime(enter_date) NOT NULL),
	description text)}
db eval {CREATE TABLE splits (
	guid text PRIMARY KEY NOT NULL,
	tx_guid text NOT NULL REFERENCES transactions (guid),
	account_guid text NOT NULL REFERENCES accounts (guid),
	memo text,
	flags integer,
	value bigint NOT NULL,
	quantity bigint NOT NULL)}
db eval {CREATE TABLE commodities (
	guid text PRIMARY KEY NOT NULL,
	mnemonic text NOT NULL,
	fullname text,
	cusip text,
	type text)}
db eval {CREATE TABLE scheduled_transactions (
	guid text PRIMARY KEY NOT NULL REFERENCES transactions (guid),
    -- The Julian day when transaction was last scheduled
	last_used double NOT NULL)}
db eval {CREATE TABLE stock_splits (
	guid text PRIMARY KEY NOT NULL,
	commodity_guid text NOT NULL REFERENCES commodities (guid),
    split_date text NOT NULL,
    split_factor real NOT NULL)}
db eval {CREATE INDEX tx_post_date_index ON transactions (post_date)}
db eval {CREATE INDEX splits_tx_guid_index ON splits (tx_guid)}
db eval {CREATE INDEX splits_account_guid_index ON splits (account_guid)}
db eval {CREATE UNIQUE INDEX unique_accounts on accounts (parent_guid, name)}
db eval {CREATE INDEX commodities_index ON commodities (guid)}
db eval {CREATE INDEX parents on accounts (parent_guid)}
db eval {CREATE INDEX price_by_commodity on prices (commodity_guid)}
db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
	values (NewUUID, 'Root', NULL, '', '', AccountFlagPlaceHolderBit|AccountFlagPermanentBit)}
db eval {insert into book (root_account_guid, name) values ((select guid from accounts where name='Root' and parent_guid is null), $bookName)}
db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
	values (NewUUID, 'Assets', (select root_account_guid from book), '', '', AccountFlagDescendentsAreAssetsBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit)}
db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
	values (NewUUID, 'Liabilities', (select root_account_guid from book), '', '', AccountFlagDescendentsAreLiabilitiesBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit)}
db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
	values (NewUUID, 'Income', (select root_account_guid from book), '', '', AccountFlagDescendentsAreIncomeBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit)}
db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
	values (NewUUID, 'Expenses', (select root_account_guid from book), '', '', AccountFlagDescendentsAreExpensesBit|AccountFlagPlaceHolderBit|AccountFlagPermanentBit)}
db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
	values (NewUUID, 'Equity', (select root_account_guid from book), '', '', AccountFlagNoChildrenBit|AccountFlagPermanentBit)}
db eval {insert into accounts (guid, name, parent_guid, code, description, flags)
	values (NewUUID, 'Unspecified', (select root_account_guid from book), '', '', AccountFlagHiddenBit|AccountFlagPermanentBit|AccountFlagNoChildrenBit)}
