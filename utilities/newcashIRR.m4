m4_include(`newcash.m4')m4_dnl
#!/usr/bin/env tclsh

package require sqlite3

source [exec which newcashCommon]

## Constants
m4_define(CusipIndex,0)m4_dnl
m4_define(DbFileIndex,1)m4_dnl
m4_define(CommandLineArgs,`cusip pathToDatabase')m4_dnl
m4_define(True,1)m4_dnl
m4_define(False,0)m4_dnl

## Check that the number of arguments is correct
if {$argc != [llength {CommandLineArgs}]} {
    puts "Usage: newcashIRR CommandLineArgs"
    puts "You supplied: $argv"
    exit 1
}

proc debugit {sql} {
    puts "Query debug: $sql"
}

## Get the arguments
set cusip [lindex $argv CusipIndex]
set dbFile [lindex $argv DbFileIndex]


## Open the database
sqlite3 db $dbFile

## For debugging
#db trace debugit

# Enable extension loading
db enable_load_extension True

# And load the math extensions
switch [exec uname] {
    OpenBSD -
    FreeBSD -
    DragonFly {set extensionsPath /usr/local/from_source/lib/libSqliteExtensions.so}
    Linux {set extensionsPath /usr/local/lib/libSqliteExtensions.so}
}
db eval {select load_extension($extensionsPath)}

# Find all the transactions since the last zero crossing.
# First, get the size of the current position
set remainder [db eval {
select quantity
from (select sum(quantity*(select ifnull(exp(sum(log(split_factor))), 1.0) 
                                       from stock_splits ss 
                                       where ss.commodity_guid = a.commodity_guid 
                                         and ss.split_date > date(t.post_date)))/MAXIMUM_DENOMINATOR as quantity 
      from commodities c , splits s, 
           transactions t, 
           ((((((accounts a left outer join accounts a2 on a.parent_guid=a2.guid) 
                left outer join accounts a3 on a2.parent_guid=a3.guid) 
                    left outer join accounts a4 on a3.parent_guid=a4.guid) 
                        left outer join accounts a5 on a4.parent_guid=a5.guid) 
                            left outer join accounts a6 on a5.parent_guid=a6.guid) 
                                left outer join accounts a7 on a6.parent_guid=a7.guid)
                                    left outer join accounts a8 on a7.parent_guid=a8.guid 
      where c.cusip = $cusip
        and a.commodity_guid = c.guid
        and s.account_guid = a.guid 
        and not (s.flags & SPLIT_FLAG_TRANSFER) 
        and s.tx_guid = t.guid 
        and (a.parent_guid isnull or not (a2.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
        and (a2.parent_guid isnull or not (a3.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
        and (a3.parent_guid isnull or not (a4.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
        and (a4.parent_guid isnull or not (a5.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
        and (a5.parent_guid isnull or not (a6.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
        and (a6.parent_guid isnull or not (a7.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
        and (a7.parent_guid isnull or not (a8.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
        and julianday(t.post_date) <= ?1 
      group by c.guid ) }]

set transactions [db eval {
select julianday(t.post_date),
       cast (s.quantity as double)*(select ifnull(exp(sum(log(split_factor))), 1.0)
                                    from stock_splits ss
                                    where ss.commodity_guid = a.commodity_guid
                                      and ss.split_date > date(t.post_date))/MAXIMUM_DENOMINATOR
from splits s, transactions t, commodities c,
   ((((((accounts a left outer join accounts a2 on a.parent_guid=a2.guid) 
        left outer join accounts a3 on a2.parent_guid=a3.guid) 
            left outer join accounts a4 on a3.parent_guid=a4.guid) 
                left outer join accounts a5 on a4.parent_guid=a5.guid) 
                    left outer join accounts a6 on a5.parent_guid=a6.guid) 
                        left outer join accounts a7 on a6.parent_guid=a7.guid)
                            left outer join accounts a8 on a7.parent_guid=a8.guid 
where c.cusip = $cusip
  and a.commodity_guid = c.guid
  and s.account_guid = a.guid
  and s.tx_guid = t.guid
  and s.quantity != 0 
  and not (s.flags & SPLIT_FLAG_TRANSFER)
  and (a.parent_guid isnull or not (a2.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
  and (a2.parent_guid isnull or not (a3.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
  and (a3.parent_guid isnull or not (a4.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
  and (a4.parent_guid isnull or not (a5.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
  and (a5.parent_guid isnull or not (a6.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
  and (a6.parent_guid isnull or not (a7.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
  and (a7.parent_guid isnull or not (a8.flags & ACCOUNT_FLAG_DESCENDENTS_NEED_COMMODITY_LINK)) 
order by t.post_date desc}]

foreach {date quantity} $transactions {
    set remainder [expr $remainder - $quantity]
    if {abs($remainder) < 0.1} {
        set lastZeroCrossing $date
        break
    }
}

if {! [info exists lastZeroCrossing]} {
    puts {Failed to find last zero crossing date}
    exit 1
    }

# Now get the transactions on or after the last zero crossing
set values [db eval {
select date(t.post_date), julianday(date(t.post_date)), -(cast(s.value as double)/MAXIMUM_DENOMINATOR), a.guid
from splits s, transactions t, commodities c, accounts a 
where julianday(t.post_date) >= $lastZeroCrossing
  and c.cusip = $cusip
  and a.commodity_guid = c.guid
  and s.account_guid = a.guid 
  and s.tx_guid = t.guid 
  and not (s.flags & SPLIT_FLAG_TRANSFER)
order by t.post_date}]

set previousDate {}
set previousJulianDay {}
set previousValue 0
set previousGuid {}
foreach {date julianDay value guid} $values {
    if {([string length $previousDate] > 0) && ($julianDay-$previousJulianDay)<3} {
        set previousValue [expr $value+$previousValue]
        continue
    } elseif {[string length $previousDate] > 0} {
        puts "$previousDate, $previousValue, [guidToPath db $previousGuid]"
    }
    set previousDate $date
    set previousJulianDay $julianDay
    set previousValue $value
    set previousGuid $guid
}

if {[string length $previousDate] > 0} {
    puts "$previousDate, $previousValue, [fullPath $previousGuid]"
}
