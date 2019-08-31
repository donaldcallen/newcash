m4_include(`newcash.m4')m4_dnl
#!/usr/bin/env tclsh

package require sqlite3
source [exec which newcashCommon]

## Constants
m4_define(EndDateIndex,0)m4_dnl
m4_define(AccountPathIndex,1)m4_dnl
m4_define(DbFileIndex,2)m4_dnl
m4_define(CommandLineArgs,`endDate accountPath pathToDatabase')m4_dnl
m4_define(True,1)m4_dnl
m4_define(False,0)m4_dnl

## Check that the number of arguments is correct
if {$argc != [llength {CommandLineArgs}]} {
    puts "Usage: newcashReconciliationAid CommandLineArgs"
    puts "You supplied: $argv"
    exit 1
}

## Get the arguments
set endDate [lindex $argv EndDateIndex]
set accountPath [lindex $argv AccountPathIndex]
set dbFile [lindex $argv DbFileIndex]


## Open the database
sqlite3 db $dbFile

## Get the requested account guid
set accountGuid [pathToGuid db $accountPath]
if {[string length $accountGuid] == 0} {
    puts stderr "Failed to find the GUID for the account you specified. Please check the account path."
    exit 1
}

set reconciledBalance [db eval {select sum(s.value)
                                from splits s, transactions t
                                where s.account_guid = $accountGuid
                                    and s.flags & SPLIT_FLAG_RECONCILED
                                    and t.guid = s.tx_guid
                                    and julianday(date(t.post_date)) <= julianday($endDate)}]
puts "Reconciled balance for transactions no later than $endDate was \$$reconciledBalance"
