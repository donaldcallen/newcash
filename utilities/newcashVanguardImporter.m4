#!/usr/bin/env tclsh
m4_include(`newcash.m4')m4_dnl

source [exec which newcashCommon]

m4_define(TRUE, 1)
m4_define(FALSE, 0)

#:Assets:Investments:Bonds and notes:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard
m4_define(DCA_TRUST_BONDS_AND_NOTES, b21804cd05b09d83ffa9a1c444297b2d)
#:Assets:Investments:Equities and derivatives:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard:International
m4_define(DCA_TRUST_EQUITIES_INTERNATIONAL, 839b3e24f1edb90ebd76cf463375a47d)
#:Assets:Investments:Equities and derivatives:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard:United States
m4_define(DCA_TRUST_EQUITIES_US, d934624c13b1ceae687fb03c69b8cfa2)
#:Assets:Investments:Cash and cash equivalents:Taxable:Donald C. Allen 2003 Revocable Trust:Vanguard
m4_define(DCA_TRUST_CASH, 95ac1e5b9762b96d7bd87b7758fb3137)
#:Expenses:Investment:Commissions:Vanguard:Donald C. Allen 2003 Revocable Trust
m4_define(DCA_TRUST_COMMISSIONS, 814341b0ddab766c884bf89e8f263bb5)

#:Assets:Investments:Bonds and notes:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard
m4_define(JSA_TRUST_BONDS_AND_NOTES, 8e93bac71299464a2964db225f9902bf)
#:Assets:Investments:Equities and derivatives:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard:Europe
m4_define(JSA_TRUST_EQUITIES_EUROPE, c765b367a40099a2bf6c3ef10ed1b901)
#:Assets:Investments:Equities and derivatives:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard:United States
m4_define(JSA_TRUST_EQUITIES_US, c98dcdcb469ce66afb044c3b89293979)
#:Assets:Investments:Cash and cash equivalents:Taxable:Joan S. Allen 2003 Revocable Trust:Vanguard
m4_define(JSA_TRUST_CASH, 4ab9d5c9999a18736f4c495210cc3c02)
#:Expenses:Investment:Commissions:Vanguard:Joan S. Allen 2003 Revocable Trust
m4_define(JSA_TRUST_COMMISSIONS, db543a9cf6e87e5b2f15223eeb6ee17e)

#:Assets:Investments:Bonds and notes:Tax-deferred:Don:Vanguard Individual IRA
m4_define(DCA_INDIVIDUAL_IRA_BONDS_AND_NOTES, 6a77cc0d486761ff737abc01a9b68aff)
#:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Individual IRA:International
m4_define(DCA_INDIVIDUAL_IRA_EQUITIES_INTERNATIONAL, 5eb50f9c8bb79e5154d6247c8fdcb753)
#:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Individual IRA:United States
m4_define(DCA_INDIVIDUAL_IRA_EQUITIES_US, 468133b246bee4019dc024295fc73731)
#:Assets:Investments:Cash and cash equivalents:Tax-deferred:Don:Vanguard Individual IRA
m4_define(DCA_INDIVIDUAL_IRA_CASH, b75cb26cc20ce1ea928fa0036185ca03)
#:Expenses:Investment:Commissions:Vanguard:DCA Individual IRA
m4_define(DCA_INDIVIDUAL_IRA_COMMISSIONS, 0b348120ab9a07e5e2a080f39fee6d46)

#:Assets:Investments:Bonds and notes:Tax-deferred:Don:Vanguard Inherited IRA
m4_define(DCA_INHERITED_IRA_BONDS_AND_NOTES, 5c6db83acf4c0c1e30183eba80c030cd)
#:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Inherited IRA:International
m4_define(DCA_INHERITED_IRA_EQUITIES_INTERNATIONAL, 9c8c5029318c2013a5a8e45526d01284)
#:Assets:Investments:Equities and derivatives:Tax-deferred:Don:Vanguard Inherited IRA:United States
m4_define(DCA_INHERITED_IRA_EQUITIES_US, 7415960ea7444842ed87f42cfbd49e3c)
#:Assets:Investments:Cash and cash equivalents:Tax-deferred:Don:Vanguard Inherited IRA
m4_define(DCA_INHERITED_IRA_CASH, f301ade58f8adff68ee021a03e11e29f)
#:Expenses:Investment:Commissions:Vanguard:DCA Inherited IRA
m4_define(DCA_INHERITED_IRA_COMMISSIONS, 025585fbe2ff1f1c8309d23918a6376e)

m4_define(DEFAULT_ASSET_PARENT_INDEX, 2)

#:Income:Investments:Taxable:Dividends
m4_define(TAXABLE_DIVIDENDS, b4049ae08bd9f4bc9826ccfa9da503b5)
#:Income:Investments:Tax-deferred:Dividends
m4_define(TAX_DEFERRED_DIVIDENDS, 5c6db83acf4c0c1e30183eba80c030cd)

#:Expenses:Tax:Foreign
m4_define(FOREIGN_TAX_EXPENSE_GUID, 3fcabaaef90cc0519f2df3b4e19eb06e)
#:Expenses:Investment:Management fees:ADR custody fees
m4_define(ADR_CUSTODY_FEE_EXPENSE_GUID, 8b2720407446a7365fe30810b7ccb09a)
#:Unspecified
m4_define(UNSPECIFIED_ACCOUNT_GUID, b1491c8019a58916d38e51c817741008)

m4_define(CSV_FILE_INDEX, 0)
m4_define(NEWCASH_DATABASE_FILE_INDEX, 1)
m4_define(COMMAND_LINE_ARGS, {csv_file newcash_database_file})

# Column indices into .csv file
m4_define(ACCOUNTNUMBERINDEX, 0)
m4_define(TRADEDATEINDEX, 1)
m4_define(SETTLEMENTDATEINDEX, 2)
m4_define(TRANSACTIONTYPEINDEX, 3)
m4_define(DESCRIPTIONINDEX, 4)
m4_define(NAMEINDEX, 5)
m4_define(SYMBOLINDEX, 6)
m4_define(QUANTITYINDEX, 7)
m4_define(COMMISSIONINDEX, 10)
m4_define(AMOUNTINDEX, 11)

package require sqlite3

## Procedures
proc maybeExpr {x} {
    if {$x == ""} {
        return [expr 0]
    } else {
        return [expr $x]
    }
}

proc insertTransaction {settlementDate description targetGuid amount quantity commission closeP} {
    global cashAccountGuid commissionsAccountGuid
    if {[catch {
        db transaction {
            # Generate a guid for the new transaction
            set transactionGuid  [oneRowOneColumn [db eval {select NEW_UUID}] getNewGuid]
            # Insert the transaction
            db eval {insert into transactions (guid, num, post_date, enter_date, description) 
                values ($transactionGuid, '',  $settlementDate||' 12:00:00', datetime('NOW', 'localtime'), $description)}
            # And the splits
            # quantity has correct sign, but amount is from the cash account point of view
            db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                values (NEW_UUID, $transactionGuid, $targetGuid,
                        '', 0, -$amount, $quantity)}
            # Commission is always positive, therefore it is stated from the perspective of the expense account. So here is always needs
            # to be subtracted from this split's amount, which is from the perspective of the cash account. 
            db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                values (NEW_UUID, $transactionGuid, $cashAccountGuid,
                        '', 0, $amount-$commission, 0)}
            if {$commission != 0} {
                # Create a split for the commission expense
                db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                    values (NEW_UUID, $transactionGuid, $commissionsAccountGuid,
                            '', 0, $commission, 0)}
            }
            if {$closeP} {
                db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                    values (NEW_UUID, $transactionGuid, $targetGuid,
                            '', 0, 0, 0)}
                db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                    values (NEW_UUID, $transactionGuid, 'UNSPECIFIED_ACCOUNT_GUID',
                            '', 0, 0, 0)}
            }
        }    
    } != TCL_OK]} {
        puts stderr "Unable to process transaction with the following description: $description.\n"
        exit 1
    } 
}

proc processTransaction {name settlementDate description parentGuid targetGuid symbol amount quantity closeP} {
    global  cashAccountGuid
    if {([llength $parentGuid] > 0) && ([string length $targetGuid] == 0)} {
        if {[llength $parentGuid] == 1} {
            set targetGuid [db eval {select a.guid
                                    from accounts a, commodities c
                                    where a.parent_guid=$parentGuid and c.mnemonic=$symbol and a.commodity_guid=c.guid}]
            if {[llength $targetGuid] == 0} {
                set targetGuid [db eval {select NEW_UUID}]
                puts "processTransaction: creating account for $name, $symbol"
                db eval {insert into accounts (guid, name, parent_guid, commodity_guid, code, description, flags)
                    values ($targetGuid, $name, $parentGuid, (select guid from commodities where mnemonic = $symbol), '', '', 0)}
            }
        } else {
            set targetGuid [findAssetGuid $symbol $name $closeP]
        }
    } elseif {! (([llength $parentGuid] == 0) && ([string length $targetGuid] > 0))} {
        puts stderr "processTransaction: invalid combination of parentGuid and targetGuid arguments: $parentGuid|$targetGuid"
        exit 1
    }
    ## Make sure we can find the commodity for this symbol
    set commodityRowCount [db eval {select count(*) from commodities where mnemonic=$symbol}]
    if {$commodityRowCount != 1} {
        puts "processTransaction: count of commodities with symbol $symbol was $commodityRowCount. Should be 1.
Name: $name, Settlement date: $settlementDate, Description: $description"
        exit 1
    }
    set commodityFlags [db eval {select ifnull(flags,0) from commodities where mnemonic=$symbol}]
    if {($commodityFlags & COMMODITY_FLAG_MONEY_MARKET_FUND) != 0} {
        set quantity [expr -$amount]
    }
    insertTransaction $settlementDate $description $targetGuid $amount $quantity 0 $closeP
}

proc findAssetGuid {symbol name closeP} {
    global assetParentGuids
    ## Search parents for a child asset that matches the symbol
    foreach parentGuid $assetParentGuids {
        set guid [db eval {select a.guid
                                    from accounts a, commodities c
                                    where parent_guid=$parentGuid
                                        and c.guid = a.commodity_guid
                                        and c.mnemonic = $symbol}]
        if {[llength $guid] == 1} {
            return $guid
        }
    }
    ## Didn't find it. Is this an opening transaction?
    if {! $closeP} {
        ## Is there a commodity for this symbol?
        set commodity_guid [db eval {select guid from commodities where mnemonic=$symbol}]
        if {[llength $commodity_guid] == 0} {
            ## No commodity present, create it
            puts "findAssetGuid: creating commodity for $symbol, $name"
            set commodity_guid [db eval {select NEW_UUID}]
            db eval {insert into commodities (guid, mnemonic, fullname, flags)
                        values ($commodity_guid, $symbol, $name, 0)}
        }
        ## Create asset account under default parent
        puts "findAssetGuid: creating asset account for $symbol, $name, $parentGuid"
        set assetAccountGuid [db eval {select NEW_UUID}]
        set parentGuid [lindex $assetParentGuids DEFAULT_ASSET_PARENT_INDEX]
        db eval {insert into accounts (guid, name, parent_guid, commodity_guid, code, description, flags)
            values ($assetAccountGuid, $name, $parentGuid, $commodity_guid, '', '', 0)}
        return $assetAccountGuid
    } else {
        puts "findAssetGuid: failed to find asset account for symbol $symbol when processing a closing transaction"
        exit 1
    }
}

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

proc debugit {sql} {
    puts "debug: $sql"
}

##Main program

## Check that the number of arguments is correct
if {$argc != [llength COMMAND_LINE_ARGS]} {
    puts "Usage: newcashVanguardImporter COMMAND_LINE_ARGS"
    exit 1
}

## Get the arguments
set csvFile [lindex $argv CSV_FILE_INDEX]
set newcashDatabaseFile [lindex $argv NEWCASH_DATABASE_FILE_INDEX]

## Open the csv file
set csvHandle [open $csvFile r]
## Open the Newcash database
sqlite3 db $newcashDatabaseFile
#db trace debugit

set line [gets $csvHandle]
## Get the account number
set splitLine [split $line ,]
set accountNumber [lindex $splitLine ACCOUNTNUMBERINDEX]

## Make sure the accountNumber is valid
set accountDescription [db eval {select description from accounts where code = $accountNumber}]
if {[llength $accountDescription] != 1} {
    puts stderr "$accountNumber is not valid"
    exit 1
} else {
    puts "Processing account $accountDescription"
}

switch $accountNumber {
    18477440 {
        set assetParentGuids {DCA_TRUST_BONDS_AND_NOTES DCA_TRUST_EQUITIES_INTERNATIONAL DCA_TRUST_EQUITIES_US}
        set cashAccountGuid DCA_TRUST_CASH
        set commissionsAccountGuid DCA_TRUST_COMMISSIONS
        set dividendsParentGuid TAXABLE_DIVIDENDS
    }
    10792723 {
        set assetParentGuids {JSA_TRUST_BONDS_AND_NOTES JSA_TRUST_EQUITIES_EUROPE JSA_TRUST_EQUITIES_US}
        set cashAccountGuid JSA_TRUST_CASH
        set commissionsAccountGuid JSA_TRUST_COMMISSIONS
        set dividendsParentGuid TAXABLE_DIVIDENDS
    }
    66996984 {
        set assetParentGuids {DCA_INDIVIDUAL_IRA_BONDS_AND_NOTES DCA_INDIVIDUAL_IRA_EQUITIES_INTERNATIONAL DCA_INDIVIDUAL_IRA_EQUITIES_US}
        set cashAccountGuid DCA_INDIVIDUAL_IRA_CASH
        set commissionsAccountGuid DCA_INDIVIDUAL_IRA_COMMISSIONS
        set dividendsParentGuid TAX_DEFERRED_DIVIDENDS
    }
    36750678 {
        set assetParentGuids {DCA_INHERITED_IRA_BONDS_AND_NOTES DCA_INHERITED_IRA_EQUITIES_INTERNATIONAL DCA_INHERITED_IRA_EQUITIES_US}
        set cashAccountGuid DCA_INHERITED_IRA_CASH
        set commissionsAccountGuid DCA_INHERITED_IRA_COMMISSIONS
        set dividendsParentGuid TAX_DEFERRED_DIVIDENDS
    }
    default {
        puts "Account number $accountNumber not supported"
        exit 1
    }
}

while {![eof $csvHandle]} {
    set settlementDate [lindex $splitLine SETTLEMENTDATEINDEX]
    set name [lindex $splitLine NAMEINDEX]
    set transactionType [lindex $splitLine TRANSACTIONTYPEINDEX]
    set quantity [maybeExpr [lindex $splitLine QUANTITYINDEX]]
    set commission [maybeExpr [lindex $splitLine COMMISSIONINDEX]]
    set amount [maybeExpr [lindex $splitLine AMOUNTINDEX]]
    set symbol [lindex $splitLine SYMBOLINDEX]
    ## Check if we got a symbol
    if {[string length $symbol] == 0} {
        if {[string compare $name {VANGUARD FEDERAL MONEY MARKET FUND}] == 0} {
            ## For some odd reason, Vanguard omits this symbol from its .csv files
            set symbol VMFXX
        }
    }

    switch -exact $transactionType {
        Sell -
        {Sell to close} -
        {Sell to open} {
            switch -exact $transactionType {
                {Sell to close} -
                {Sell to open} {
                    set quantity [expr $quantity*100]
                }
            }
            switch -exact $transactionType {
                {Sell to close} -
                Sell {
                    set closeP TRUE
                }
                default {
                    set closeP FALSE
                }
            }
            set assetGuid [findAssetGuid $symbol $name $closeP]
            insertTransaction $settlementDate "$transactionType $name" $assetGuid [expr $amount+$commission] $quantity $commission $closeP
        }
        Assignment -
        Expired -
        Buy -
        {Buy to open} -
        {Buy to close} {
            switch -exact $transactionType {
                Assignment -
                Expired -
                {Buy to open} -
                {Buy to close} {
                    ## It's an option
                    set quantity [expr $quantity*100]
                }

            }
            switch -exact $transactionType {
                {Buy to open} -
                Buy {
                    set closeP FALSE
                }
                default {
                    set closeP TRUE
                }
            }
            set assetGuid [findAssetGuid $symbol $name $closeP]
            insertTransaction $settlementDate "$transactionType $name" $assetGuid [expr $amount+$commission] $quantity $commission $closeP
        }
        {Dividend (adjustment)} -
        Dividend {
            processTransaction $name $settlementDate "$name dividend" $dividendsParentGuid {} $symbol $amount 0.0 FALSE
        }
        {Sweep out} -
        {Sweep in} {
            processTransaction $name $settlementDate $transactionType $assetParentGuids {} $symbol $amount [expr -$amount] FALSE
        }
        Reinvestment {
            processTransaction $name $settlementDate "Re-invest $name dividend" $assetParentGuids {} $symbol $amount $quantity FALSE
        }
        Withholding {
            processTransaction $name $settlementDate "Foreign tax withheld ($name)" {} FOREIGN_TAX_EXPENSE_GUID $symbol $amount 0 FALSE
        }
        Fee {
            processTransaction $name $settlementDate "ADR custody fee ($name)" {} ADR_CUSTODY_FEE_EXPENSE_GUID $symbol $amount 0 FALSE
        }
        default {
            puts stderr "Unable to process transaction of type $transactionType, settlement date $settlementDate, name $name"
        }
    }
    set line [gets $csvHandle]
    set splitLine [split $line ,]
}
