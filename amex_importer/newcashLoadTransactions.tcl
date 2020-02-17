#!/usr/bin/env tclsh

package require sqlite3

## Constants
set commandLineArgs {pathToLevenshteinLib pathToNewcashDatabase}
set extensionIndex 0
set dbFileIndex 1

## Procedures
proc converttoISO9601 {usDate} {
    set splitDate [split $usDate /]
    set year [lindex $splitDate 2]
    if {[string length $year]==2} {
      set year 20$year
    }
    set month [lindex $splitDate 0]
    if {[string length $month]==1} {
      set month 0$month
    }
    set day [lindex $splitDate 1]
    if {[string length $day]==1} {
      set day 0$day
    }
    return $year-$month-$day
}

proc sqltrace {sql} {
    puts "debug: $sql"
}

## Main program
## Check that the number of arguments is correct
if {$argc != [llength $commandLineArgs]} {
    puts "Wrong number of arguments on the command line. Should be [llength $commandLineArgs]. You supplied: $argc"
    puts "Usage: newcashLoadTransactions $commandLineArgs"
    exit 1
    }

## Get the arguments
set dbFilePath [lindex $argv $dbFileIndex]

## Open the newcash database
sqlite3 db $dbFilePath

## Sqlite debug
#db trace sqltrace

# And load the Levenshtein extension
db enable_load_extension True
set extensionPath [lindex $argv $extensionIndex]
db eval {select load_extension($extensionPath)}
db enable_load_extension False

## Unspecified account guid
set unspecifiedGuid b1491c8019a58916d38e51c817741008
## AMEX account guid
set amexAccountGuid 86ff3f45c73fc1de79e6f5acf2128f4f

while {[gets stdin line] >= 0} {
    set splitLine [split $line ,]
    set postDate [converttoISO9601 [lindex $splitLine 0]]
    set description [lindex $splitLine 1]
    set amount [lindex $splitLine 2]
    set descriptionLength [string length $description]

    ## Check to see if this is a duplicate
    set dup [db eval {select t.guid from transactions t, splits s where s.account_guid=$amexAccountGuid and s.tx_guid=t.guid and
                         t.post_date=$postDate and t.description=$description and s.value=$amount}]
    if {[llength $dup] == 0} {
        set match [db eval {select t.guid
            from transactions t, splits s
            where s.tx_guid = t.guid and s.account_guid = $amexAccountGuid
            and (cast (levenshtein(description, $description) as double))/(cast ($descriptionLength as double)) < 0.4 order by post_date desc}]
        if {([llength $match] >= 0) && ([string length [lindex $match 0]] > 0)} {
            set transactionGuid [lindex $match 0]
            puts "Match transactionGuid $transactionGuid"
            set balancingSplitAccountGuid [db eval {select account_guid from splits
                                            where tx_guid = $transactionGuid and
                                                    account_guid != $amexAccountGuid}]
            if {[llength $balancingSplitAccountGuid] != 1} {
                ## More than one split. Can't process.
                set balancingSplitAccountGuid $unspecifiedGuid
            }
        } else {
            set balancingSplitAccountGuid $unspecifiedGuid
        }
        ## Insert transaction
        set newTransactionGuid [db eval {select lower(hex(randomblob(16)))}]
        db eval {insert into transactions
                    (guid, num, post_date, enter_date, description)
                    values ($newTransactionGuid, '', $postDate, datetime('now', 'localtime'), $description)}
        db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
            values (lower(hex(randomblob(16))), $newTransactionGuid, $amexAccountGuid, '', 0, -$amount,  0)}
        db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity)
            values (lower(hex(randomblob(16))), $newTransactionGuid, $balancingSplitAccountGuid, '', 0, $amount,  0)}
    }
}
