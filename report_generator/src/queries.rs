pub const MARKETABLE_ASSET_VALUE_SQL: &str = concat!(
                                                     "
select case when svq.split_quantity < ",
                                                     constants!(EPSILON),
                                                     "
         then 0.
         else
           case when p.price isnull
             then svq.split_value
             else svq.split_quantity*p.price
           end
         end as value
from accounts a,
    (select ifnull(sum(value), 0.) as split_value,
            ifnull(sum(quantity*(select ifnull(exp(sum(log(split_factor))), 1.0)
                                  from stock_splits ss
                                  where ss.commodity_guid = a.commodity_guid
                                    and ss.split_date > date(t.post_date))), 0.0) as split_quantity
     from splits s, transactions t, accounts a
     where s.account_guid = ?1 and a.guid = s.account_guid and s.tx_guid = t.guid
        and julianday(t.post_date) <= ?2
    ) svq,
    (select avg(ifnull(value, 0)) as price
     from prices p,
       (select p.commodity_guid, max(timestamp) as max_price_date
        from prices p, accounts a
        where julianday(timestamp) <= ?2 and p.commodity_guid = a.commodity_guid and a.guid = ?1
       ) pd
     where p.commodity_guid=pd.commodity_guid and p.timestamp=pd.max_price_date
    ) p
where a.guid = ?1"
);

pub const NON_MARKETABLE_ASSET_AND_LIABILITY_VALUE_SQL: &str = "
select ifnull(svq.split_value,0)
from accounts a, (select sum(value) as split_value
                  from splits s, transactions t
                  where s.account_guid = ?1 and s.tx_guid = t.guid
                    and julianday(t.post_date) <= ?2
                 ) svq
where a.guid = ?1";

pub const INCOME_AND_EXPENSES_VALUE_SQL: &str = "
select ifnull(svq.split_value,0)
from accounts a, (select sum(value) as split_value
                  from splits s, transactions t
                  where s.account_guid = ?1
                    and s.tx_guid = t.guid
                    and julianday(t.post_date) <= ?2
                    and julianday(t.post_date) >= ?3
               ) svq
where a.guid = ?1";

pub const OPEN_POSITIONS_SQL: &str = concat!(
                                             "
select guid, mnemonic, fullname, quantity
from (select c.guid, c.mnemonic, c.fullname,
        sum(quantity*(select ifnull(exp(sum(log(split_factor))), 1.0)
                       from stock_splits ss where ss.commodity_guid =
                        a.commodity_guid and ss.split_date >
                            date(t.post_date))) as quantity
      from commodities c , splits s,
           transactions t,
           (((((((accounts a left outer join accounts a2 on a.parent_guid=a2.guid)
                left outer join accounts a3 on a2.parent_guid=a3.guid)
                    left outer join accounts a4 on a3.parent_guid=a4.guid)
                        left outer join accounts a5 on a4.parent_guid=a5.guid)
                            left outer join accounts a6 on a5.parent_guid=a6.guid)
                                left outer join accounts a7 on a6.parent_guid=a7.guid)
                                    left outer join accounts a8 on a7.parent_guid=a8.guid)
                                        left outer join accounts a9 on a8.parent_guid=a9.guid
      where c.guid = a.commodity_guid
          and s.account_guid = a.guid
          and not (s.flags & ",
                                             constants!(SPLIT_FLAG_TRANSFER),
                                             ")
          and s.tx_guid = t.guid
          and ((a2.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             ")
              or (a3.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             ")
              or (a4.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             ")
              or (a5.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             ")
              or (a6.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             ")
              or (a7.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             ")
              or (a8.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             ")
              or (a9.flags & ",
                                             constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                             "))
          and julianday(t.post_date) <= ?1
      group by c.guid )
where abs(quantity) > ",
                                             constants!(EPSILON),
                                             ""
);

pub const MOST_RECENT_ZERO_CROSSING_SQL: &str = concat!(
    "
select julianday(t.post_date),
       s.quantity*(select ifnull(exp(sum(log(split_factor))), 1.0)
                                    from stock_splits ss
                                    where ss.commodity_guid = a.commodity_guid
                                      and ss.split_date > date(t.post_date)), s.guid
from splits s, transactions t,
       (((((((accounts a left outer join accounts a2 on a.parent_guid=a2.guid)
            left outer join accounts a3 on a2.parent_guid=a3.guid)
                left outer join accounts a4 on a3.parent_guid=a4.guid)
                    left outer join accounts a5 on a4.parent_guid=a5.guid)
                        left outer join accounts a6 on a5.parent_guid=a6.guid)
                            left outer join accounts a7 on a6.parent_guid=a7.guid)
                                left outer join accounts a8 on a7.parent_guid=a8.guid)
                                    left outer join accounts a9 on a8.parent_guid=a9.guid
where a.commodity_guid = ?1
  and s.account_guid = a.guid
  and s.tx_guid = t.guid
  and s.quantity != 0
  and not (s.flags & ",
    constants!(SPLIT_FLAG_TRANSFER),
    ")
  and ((a2.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    ")
      or (a3.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    ")
      or (a4.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    ")
      or (a5.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    ")
      or (a6.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    ")
      or (a7.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    ")
      or (a8.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    ")
      or (a9.flags & ",
    constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
    "))
  and julianday(t.post_date) <= ?2
order by t.post_date desc"
);

pub const GET_POSITION_BASIS_SQL: &str = concat!(
                                                 "
select s.quantity*(select ifnull(exp(sum(log(split_factor))), 1.0)
                                    from stock_splits ss
                                    where ss.commodity_guid = a.commodity_guid
                                      and ss.split_date > date(t.post_date)),
                                        s.value, s.guid
from splits s, transactions t,
       (((((((accounts a left outer join accounts a2 on a.parent_guid=a2.guid)
            left outer join accounts a3 on a2.parent_guid=a3.guid)
                left outer join accounts a4 on a3.parent_guid=a4.guid)
                    left outer join accounts a5 on a4.parent_guid=a5.guid)
                        left outer join accounts a6 on a5.parent_guid=a6.guid)
                            left outer join accounts a7 on a6.parent_guid=a7.guid)
                                left outer join accounts a8 on a7.parent_guid=a8.guid)
                                    left outer join accounts a9 on a8.parent_guid=a9.guid
where julianday(t.post_date) >= ?2
  and julianday(t.post_date) <= ?3
  and a.commodity_guid = ?1
  and s.account_guid = a.guid
  and s.tx_guid = t.guid
  and quantity != 0
  and ((a2.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 ")
      or (a3.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 ")
      or (a4.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 ")
      or (a5.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 ")
      or (a6.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 ")
      or (a7.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 ")
      or (a8.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 ")
      or (a9.flags & ",
                                                 constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_ASSETS),
                                                 "))
  and not (s.flags & ",
                                                 constants!(SPLIT_FLAG_TRANSFER),
                                                 ")
order by t.post_date"
);

pub const PRICE_SQL: &str = "
select ifnull(p.value, 0.0), julianday(p.timestamp)
from prices p, (
        select max(timestamp) as max_price_date
        from prices
        where commodity_guid = ?1 and julianday(timestamp) <= ?2
        ) pd
where p.commodity_guid = ?1 and p.timestamp = pd.max_price_date";

pub const CONVERT_JULIAN_DAY_SQL: &str = "
select datetime(?1, 'localtime')";

pub const DIVIDEND_SQL: &str = concat!(
                                       "
select ifnull(-sum(s.value), 0.0)
from splits s, transactions t,
       (((((((accounts a left outer join accounts a2 on a.parent_guid=a2.guid)
            left outer join accounts a3 on a2.parent_guid=a3.guid)
                left outer join accounts a4 on a3.parent_guid=a4.guid)
                    left outer join accounts a5 on a4.parent_guid=a5.guid)
                        left outer join accounts a6 on a5.parent_guid=a6.guid)
                            left outer join accounts a7 on a6.parent_guid=a7.guid)
                                left outer join accounts a8 on a7.parent_guid=a8.guid)
                                    left outer join accounts a9 on a8.parent_guid=a9.guid
where a.commodity_guid = ?1
  and s.account_guid = a.guid
  and ((a2.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       ")
      or (a3.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       ")
      or (a4.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       ")
      or (a5.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       ")
      or (a6.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       ")
      or (a7.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       ")
      or (a8.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       ")
      or (a9.flags & ",
                                       constants!(ACCOUNT_FLAG_DESCENDENTS_ARE_INCOME),
                                       "))
  and t.guid  = s.tx_guid
  and julianday(t.post_date) >= ?2 and julianday(t.post_date) <= ?3"
);

pub const ROOT_DATA_SQL: &str = "select name, guid, flags
                                        from accounts
                                        where guid = (select root_account_guid from book)";

pub const JULIAN_CONVERSION_SQL: &str = "select julianday(?1)";

pub const ACCOUNT_CHILDREN_SQL: &str = "select name, guid, flags
                                                from accounts where parent_guid = ?1";
