#!/usr/bin/env tclsh
m4_include(`newcash.m4')m4_dnl

package require sqlite3

proc debugit {sql} {
    puts "debug: $sql"
    puts [db errorcode]
}

## Constants
set quoteFileIndex 0
set dbFileIndex 1
set commandLineArgs {pathToQuoteFile pathToDatabase}

## Check that the number of arguments is correct
if {$argc != [llength $commandLineArgs]} {
    puts "Usage: newcashGetQuotes $commandLineArgs"
    exit 1
}

## Get the arguments
set quoteFilePath [lindex $argv $quoteFileIndex]
set dbFilePath [lindex $argv $dbFileIndex]

# Open the quote file
set quoteFile [open $quoteFilePath r]

## Open the database
sqlite3 db $dbFilePath
#db trace debugit

while {[gets $quoteFile row] >= 0} {
    set splitRow [split $row \t]
    set symbol [lindex $splitRow 0]
    set cusip [lindex $splitRow 1]
    set name [lindex $splitRow 2]
    set price [lindex $splitRow 4]
    if {([string compare $price {#N/A}] == 0) || ([string length $price] == 0)} {
        puts "$name did not have a price. Skipping ..."
    } elseif {[string length $cusip] == 0} {
        puts "$name did not have a cusip. Skipping ..."
    } else {
        db eval {insert into prices (guid, commodity_guid, timestamp, value)
                   select NEW_UUID, (select guid from commodities where cusip=$cusip),
                        datetime('now', 'localtime'), $price}
    }
}
