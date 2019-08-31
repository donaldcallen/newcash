function fix_date(the_date) {
    year = substr(the_date, 7, 4);
    month = substr(the_date, 1, 2);
    day = substr(the_date, 4, 2);
    return (year"-"month"-"day)
}
    
BEGIN {
    begin_processing = 0;
    FS = ",";
}

$2 == "Trade Date" {
    begin_processing = 1;
    next;
}

$7 == "RDS B" {
    $7 = "RDS.B";
}

$7 == "RDS A" {
    $7 = "RDS.A";
}

/^[^,]/ {
    if (begin_processing) {
        printf("%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,\n", $1,fix_date($2),fix_date($3),$4,$4,$6,$7,$8,$9,$10,$11,$12,$13,$14);
    }
}
