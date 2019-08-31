pub const NEW_UUID_SQL: &str = concat!("select ", constants!(NEW_UUID));
pub const GUID_TO_PATH_SQL: &str = "
    select name, parent_guid
    from accounts
    where guid = ?1 and guid != (select root_account_guid from book)";
pub const INHERITED_P_SQL: &str = "
    select a1.guid, a1.flags
    from accounts a1, accounts a2
    where a2.guid=?1 and a1.guid=a2.parent_guid";
