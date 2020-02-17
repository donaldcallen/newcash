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

pub const ROOT_DATA_SQL: &str = "select name, guid, flags
                                        from accounts
                                        where guid = (select root_account_guid from book)";

pub const JULIAN_CONVERSION_SQL: &str = "select julianday(?1)";

pub const ACCOUNT_CHILDREN_SQL: &str = "select name, guid, flags
                                                from accounts where parent_guid = ?1";
