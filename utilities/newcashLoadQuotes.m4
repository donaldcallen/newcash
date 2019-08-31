m4_include(`newcash.m4')m4_dnl
#!/usr/bin/env tclsh

package require sqlite3

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

while {[gets $quoteFile row] >= 0} {
    set splitRow [split $row \t]
    set symbol [lindex $splitRow 0]
    set price [lindex $splitRow 3]
    if {[string compare $price {#N/A}] == 0} {
        puts "$symbol did not have a price. Skipping ..."
    } else {
        db eval {insert into prices (guid, commodity_guid, timestamp, value)
                   select lower(hex(randomblob(16))), (select guid from commodities where mnemonic=$symbol), 
                        datetime('now', 'localtime'), $price}
    }
}


