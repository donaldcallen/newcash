#!/usr/bin/env tclsh
m4_include(`newcash.m4')m4_dnl

##source [exec which newcashCommon]

## Indices into the paths lists defined below
m4_define(AssetParentGuid,c369abf6ec2c7222e3fdd174ce2c0c9a)m4_dnl
m4_define(CashAccountGuid,746ba969f732c46f37cb8577dc4bbd3e)m4_dnl
m4_define(CommissionExpenseGuid,badb70f1ddca3867d59e15cd12429b2a)m4_dnl
m4_define(UnspecifiedAccountGuid,b1491c8019a58916d38e51c817741008)m4_dnl

m4_define(TdFileIndex,0)m4_dnl
m4_define(NewcashDatabaseFileIndex,1)m4_dnl
m4_define(CommandLineArgs,{tdFile newcashDatabaseFile})m4_dnl
m4_define(True,1)m4_dnl
m4_define(False,0)m4_dnl
# Column indices into .csv file
m4_define(TradeDateIndex,0)m4_dnl
m4_define(DescriptionIndex,2)m4_dnl
m4_define(QuantityIndex,3)m4_dnl
m4_define(SymbolIndex,4)m4_dnl
m4_define(PriceIndex,5)m4_dnl
m4_define(AmountIndex,7)m4_dnl
# Column index into split description
m4_define(TransactionTypeIndex,0)m4_dnl

## Capital gain accounts
set capitalGainAccount(AAPL) 4E40BB826AB94B68B7AE6BA44808C0B2
set capitalGainAccount(AMZN) ac20afa5d19aca7722721cac8db0d194
set capitalGainAccount(BRKB) 1b780697153636ce0d0d8eb1a21b38e8
set capitalGainAccount(FB) 46271b40f5a29d8c0364346905fe9daf
set capitalGainAccount(GOOG) 42d45dd0b2161cb3fbc3fbee58917c6f
set capitalGainAccount(IBM) 13da0f66a9ab8fb68605703ba5f728fe
set capitalGainAccount(NFLX) 311c000284137f2af46c25b34b3787a4
set capitalGainAccount(SPX) 8bcf518bac0a8630311d4cbbf383a299
set capitalGainAccount(SPXW) 8bcf518bac0a8630311d4cbbf383a299
set capitalGainAccount(TSLA) 79a68c801306e37e96d1fbde9ae5f295 
set capitalGainAccount(HD) faa886e84aa4451fa09d67b610b22101 
set capitalGainAccount(JNJ) a0f26f3b76a98227de616220dd885e8c
set capitalGainAccount(MMM) 23933116469982596c352c03656caa7f
set capitalGainAccount(BLK) f7004edb897130ede9f4b51568eafeb6
set capitalGainAccount(CAT) b98ec542da3d0ebaa27a2321fa18c19e
set capitalGainAccount(MSFT) aa667f0f519832400dcfd08c6587e7ba
set capitalGainAccount(JPM) 83b94ef9e4b2a69b19777436e6962331

package require sqlite3

## Procedures
proc getCapitalGainsGuid {underLying} {
    global capitalGainAccount
    if {![info exists capitalGainAccount($underLying)]} {
        puts "Error: cound not find capital gain account for $underLying."
        exit 1
    }
    return $capitalGainAccount($underLying)
}

proc insertTransaction {tradeDate description targetGuid netAmount grossAmount quantity closingP capitalGainAccountGuid} {
    if {$closingP} {
        set expanded_description "$description (to close)"
    } else {
        set expanded_description "$description (to open)"
    }
    if {[catch {
        db transaction {
            # Generate a guid for the new transaction
            set transactionGuid  [oneRowOneColumn [db eval {select NEW_UUID}] getNewGuid]
            # Insert the transaction
            db eval {insert into transactions (guid, num, post_date, enter_date, description) 
                values ($transactionGuid, '',  $tradeDate||' 12:00:00', datetime('NOW', 'localtime'), $expanded_description)}
            # And the splits
            db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                values (NEW_UUID, $transactionGuid, $targetGuid,
                        '', 0,  $grossAmount,  $quantity)}
            db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                values (NEW_UUID, $transactionGuid, 'CashAccountGuid',
                        '', 0,  $netAmount, 0)}
            set commission [expr round(abs($grossAmount+$netAmount)*100.0)/100.0]
            if {$commission != 0} {
                # Create a split for the commission expense
                db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                    values (NEW_UUID, $transactionGuid, 'CommissionExpenseGuid',
                            '', 0,  $commission, 0)}
            }
            if {$closingP} {
                db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                    values (NEW_UUID, $transactionGuid, $targetGuid,
                            '', 0, 0, 0)}
                db eval {insert into splits (guid, tx_guid, account_guid, memo, flags, value, quantity) 
                    values (NEW_UUID, $transactionGuid, $capitalGainAccountGuid,
                            '', 0, 0, 0)}
            }
        }    
    } != TCL_OK]} {
        puts stderr "Unable to process transaction with the following description: $description.\n"
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

proc getAssetAccountInfo {name commodityGuid} {
    ## Do we have an asset account?
    set found False
    db eval {select a.guid as assetGuid 
                from accounts a, commodities c
                where parent_guid='AssetParentGuid'
                    and c.guid = a.commodity_guid
                    and c.fullname=$name} {
        set found True
    }
    if {! $found} {
        ## No, create it.
        set assetGuid [db eval {select NEW_UUID}]
        db eval {insert into accounts (guid, name, parent_guid, commodity_guid, code, description, flags)
             values ($assetGuid, $name, 'AssetParentGuid', $commodityGuid, '', '', 0)}
        set closeP False
    } else {
        set closeP True
    }
    return [list $assetGuid $closeP]
}

proc getCommodityGuid {name} {
    ## Do we have a commodities table entry for this option?
    set found False
    db eval {select guid as commodityGuid from commodities where fullname = $name} {
        set found True
    }
    if {! $found} {
        ## No, create one.
        set commodityGuid [db eval {select NEW_UUID}]
        db eval {insert into commodities (guid, mnemonic, fullname, cusip, flags)
             values ($commodityGuid, '', $name, '', 0)}
    } 
    return $commodityGuid
}
proc debugit {sql} {
    puts "debug: $sql"
    puts [db errorcode]
}

##Main program

## Check that the number of arguments is correct
if {$argc != [llength CommandLineArgs]} {
    puts [concat {Usage: newcashTDAmeritradeImporter } CommandLineArgs]
    exit 1
}

## Get the arguments
set tdFile [lindex $argv TdFileIndex]
set newcashDatabaseFile [lindex $argv NewcashDatabaseFileIndex]

## Open the TD Ameritrade file
set tdHandle [open $tdFile r]

## Open the Newcash database
sqlite3 db $newcashDatabaseFile
#db trace debugit

while {[gets $tdHandle line] >= 0} {
    puts "Entry: $line"
    set splitLine [split $line ,]
    set tradeDate [lindex $splitLine TradeDateIndex]
    set description [lindex $splitLine DescriptionIndex]
    set splitDescription [split $description { }]
    set transactionType [lindex $splitDescription TransactionTypeIndex]
    set name [lindex $splitLine SymbolIndex]
    set price [lindex $splitLine PriceIndex]
    set netAmount [lindex $splitLine AmountIndex]
    ## The data from TD always has positive quantities, but has negative net amounts for buys, positive for sales.
    ## The amount given by TD is net of commissions, so that is the amount that goes in the cash split.
    ## The gross amount = price * quantity (in shares; TD gives the quantity in contracts).
    ## The commission is the difference between the gross and net amounts. TD provides a commission
    ## but doesn't always provide fees.
    ## The idea is to compute the quantities needed in the splits from the TD data and provide them
    ## with the signs needed by the splits.
    switch -exact $transactionType {
        Sold {
            set splitName [split $name { }]
            set underLying [lindex $splitName 0]
            set capitalGainAccountGuid [getCapitalGainsGuid $underLying]
            set commodityGuid [getCommodityGuid $name]
            set temp [getAssetAccountInfo $name $commodityGuid]
            set assetGuid [lindex $temp 0]
            set closeP [lindex $temp 1]
            ## The TD quantity is positive, in contracts. Needs to be negative, in shares.
            set quantity [expr -[lindex $splitLine QuantityIndex] * 100]
            ## The net amount provided by TD is positive, which is correct for the cash split in a sale. So
            ## nothing needs to be done about that here.
            ## The gross amount, which goes in the asset split, needs to be negative for a sale.
            set grossAmount [expr $price*$quantity]
            insertTransaction $tradeDate $description $assetGuid $netAmount $grossAmount $quantity $closeP $capitalGainAccountGuid
        }
        Bought {
            set splitName [split $name { }]
            set underLying [lindex $splitName 0]
            set capitalGainAccountGuid [getCapitalGainsGuid $underLying]
            set commodityGuid [getCommodityGuid $name]
            set temp [getAssetAccountInfo $name $commodityGuid]
            set assetGuid [lindex $temp 0]
            set closeP [lindex $temp 1]
            ## The TD quantity is positive, in contracts. Needs to be in shares.
            set quantity [expr [lindex $splitLine QuantityIndex] * 100]
            ## The net amount is negative for a purchase, which is correct. But stick it in an expr anyway,
            ## so sqlite treats it as number. Got to rewrite this in rust. This language is awful.
            set netAmount [expr $netAmount]
            ## The gross amount, which goes in the asset split, needs to be positive for a purchase.
            set grossAmount [expr $price*$quantity]
            insertTransaction $tradeDate $description $assetGuid $netAmount $grossAmount $quantity $closeP $capitalGainAccountGuid
        }
        REMOVAL {
            ## For some inexplicable reason, TD doesn't include the symbol of the expired option. Try to divine it.
            if {[string length $name] == 0} {
                # In this situation, the description looks like this:
                # REMOVAL OF OPTION DUE TO EXPIRATION (0SPXW.US82630000)
                if {[regexp {\(0([A-Z0-9]+)\.+[A-Z].*([0-9]{8})\)} $description dont_care underLying yearAndStrike]} {
                    # Try to find a unique commodity with the underlying and strike_price in its name
                    set yearDigit [string index $yearAndStrike 0]
                    switch -exact $yearDigit {
                        7 -
                        8 -
                        9 {set year 201$yearDigit}
                        default {set year 202$yearDigit}
                    }
                    set strikePrice [string trimleft [string range $yearAndStrike 1 4] 0]
                    set commodityInfo [db eval {select guid, fullname from commodities
                                            where fullname like $underLying || '%' || $year || '%' || $strikePrice || '%'}]
                    ## Die if we did not find exactly one
                    if {[llength $commodityInfo] != 2} {
                        puts stderr "Failed to locate commodity"
                        exit 1
                    }
                    set commodityGuid [lindex $commodityInfo 0]
                    set name [lindex $commodityInfo 1]
                } else {
                    puts stderr "regexp failed"
                    exit 1
                }

            } else {
                set splitName [split $name { }]
                set underLying [lindex $splitName 0]
                set commodityGuid [getCommodityGuid $name]
            }
            set capitalGainAccountGuid [getCapitalGainsGuid $underLying]
            set temp [getAssetAccountInfo $name $commodityGuid]
            set assetGuid [lindex $temp 0]
            set closeP [lindex $temp 1]
            ## TD inexplicably doesn't indicate whether the expired option was a long or short position.
            ## So we need to run a query to determine that. We negate the current quantity in the query
            ## so that it returns a quantity for this transaction that has the correct sign
            if {!$closeP} {
                puts "Error: removal transaction not a closing transaction: $description"
                exit 1
            }
            set found False
            db eval {
                select -s.quantity as quantity
                    from transactions t, splits s 
                    where s.account_guid = $assetGuid
                       and s.tx_guid = t.guid 
                       and ((s.quantity != 0) or ((s.quantity = 0) and 
                           (select count(*)
                            from splits s
                            where s.tx_guid = t.guid and s.account_guid = $assetGuid) = 1)) 
                    order by post_date, enter_date} {
                insertTransaction $tradeDate $description $assetGuid 0 0 $quantity True $capitalGainAccountGuid
                set found True
            }
            if {!$found} {
                puts "Error: processing removal transaction, but found no existing position: $description"
                exit 1
            }
        }
        
        default {
            puts stderr "Unable to process transaction of type $transactionType, settlement date $tradeDate, name $name"
        }
    }
}
