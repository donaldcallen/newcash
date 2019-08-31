m4_include(`newcash.m4')m4_dnl
#!/usr/bin/env tclsh

package require sqlite3

## Constants
set dateIndex 0
set numIndex 1
set descriptionIndex 2
set principalMemoIndex 3
set interestMemoIndex 4
set paymentIndex 5
set rateIndex 6
set paymentsPerYearIndex 7
set minimumPeriodIndex 8
set dbFileIndex 9

set commandLineArgs {date num description principal-memo interest-memo payment-memo annual-interest-rate payments-per-year minimum-period path-to-database}

## Queries
set getTemplateTransactionGuid {select guid from transactions 
    where date(post_date)=$date 
        and num=$num
        and description=$description}
set getTemplatePrincipalSplitGuid {select guid from splits where memo=$principalMemo and tx_guid=$templateTransactionGuid}
set getTemplateInterestSplitGuid {select guid from splits where memo=$interestMemo and tx_guid=$templateTransactionGuid}
set getCurrentPrincipal {select sum(value) from splits where account_guid = (select account_guid from splits where guid=$templatePrincipalSplitGuid)}
set getPayment {select value from splits where memo=$paymentMemo and tx_guid=$templateTransactionGuid}
set getNewGuid {select NEW_UUID}
set getDaysSinceLast {select (julianday('NOW')-last_used) 
    from scheduled_transactions 
    where guid=$templateTransactionGuid}

## Procedures
proc oneRowOneColumn {result sql} {
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

proc maybeOneRowOneColumn {result sql} {
    switch [llength $result] {
        0 -
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
if {$argc != [llength $commandLineArgs]} {
    puts "Usage: newcashScheduledLoanPayment $commandLineArgs"
    exit 1
}

## Get the arguments
set date [lindex $argv $dateIndex]
set num [lindex $argv $numIndex]
set description [lindex $argv $descriptionIndex]
set principalMemo [lindex $argv $principalMemoIndex] 
set interestMemo [lindex $argv $interestMemoIndex] 
set paymentMemo [lindex $argv $paymentIndex]
set paymentsPerYear [lindex $argv $paymentsPerYearIndex]
set periodRate [expr [lindex $argv $rateIndex]/100.0/$paymentsPerYear]
set minimumPeriod [lindex $argv $minimumPeriodIndex]
set dbFile [lindex $argv $dbFileIndex]

## Open the database
sqlite3 db $dbFile

## Get template transaction guid
set templateTransactionGuid [oneRowOneColumn [db eval $getTemplateTransactionGuid] $getTemplateTransactionGuid]

switch -regexp [llength $templateTransactionGuid] {
    0 { puts stderr {Specified template transaction not found.}; exit 1}
    [^1] { puts stderr {Template specification matches more than one transaction.}; exit 1}
}

if 0 { Is there an entry for this guid in the scheduled_transactions table? If so, is it more than MinimumPeriod days old?
 If the answer to the first question is 'no', proceed. If the answer to the first is 'yes' and the second is 'yes',
 proceed. Otherwise, do nothing. This allows this program to be invoked multiple times by cron without inserting duplicate
 transactions.}
set daysSinceLast [maybeOneRowOneColumn [db eval $getDaysSinceLast] $getDaysSinceLast]

if {([llength $daysSinceLast] == 0) || ([lindex $daysSinceLast 0] > $minimumPeriod)} {
    # We aren't trying to do this too soon. Get the quids of the principal and interest splits, based on the template memo fields
    set templatePrincipalSplitGuid [oneRowOneColumn [db eval $getTemplatePrincipalSplitGuid] $getTemplatePrincipalSplitGuid]
    set templateInterestSplitGuid [oneRowOneColumn [db eval $getTemplateInterestSplitGuid] $getTemplateInterestSplitGuid]
    set currentPrincipal [oneRowOneColumn [db eval $getCurrentPrincipal] $getCurrentPrincipal]
    set payment [oneRowOneColumn [db eval $getPayment] $getPayment]
    ## currentPrincipal will be negative and the payments to principal and interest are positive, so reverse the signs
    set interestPayment [expr -$currentPrincipal*$periodRate]
    set principalPayment [expr -($payment+$interestPayment)]
    
    db transaction {
    if 0 {Do the copy of the template transaction within a sqlite3 transaction
        to be sure the whole thing completes without error. If it does,
        commit. If not, roll back.}
    # Generate a guid for the new transaction
    set transactionGuid  [oneRowOneColumn [db eval $getNewGuid] $getNewGuid]
    
    # Copy the transaction
    db eval {insert into transactions (guid, num, post_date, enter_date, description) 
        select $transactionGuid, '', datetime('NOW', 'localtime'), datetime('NOW', 'localtime'), description 
        from transactions where guid=$templateTransactionGuid}
    
    # Copy the splits
    # First the principal split. 
    db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
        select NEW_UUID, $transactionGuid, account_guid, '', 0, $principalPayment, quantity
        from splits where guid=$templatePrincipalSplitGuid}
    
    # now the interest split
    db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
        select NEW_UUID, $transactionGuid, account_guid, '', 0, $interestPayment, quantity
        from splits where guid=$templateInterestSplitGuid}

    # And any others
    db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
        select NEW_UUID, $transactionGuid, account_guid, '', 0, value, quantity
        from splits where tx_guid=$templateTransactionGuid and guid != $templatePrincipalSplitGuid and guid != $templateInterestSplitGuid}
    
    
    # If we get here, record the timestamp of making the copy of the template
    switch [llength $daysSinceLast] {
        0 {db eval {insert into scheduled_transactions (guid, last_used) values ($templateTransactionGuid, julianday('NOW'))}}
        1 {db eval {update scheduled_transactions set last_used = julianday('NOW') where guid=$templateTransactionGuid}}
        default {puts {List length of daysSinceLast not 0 or 1. Should be impossible} ; exit 1}
        }
    }
}
