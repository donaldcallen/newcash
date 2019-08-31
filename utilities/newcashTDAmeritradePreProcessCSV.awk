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

/^DATE,TRANSACTION ID/ {
    begin_processing = 1;
    next;
}

/^***END OF FILE***/ {
    begin_processing = 0;
    next;
}

{
    if (begin_processing) {
        printf("%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n", fix_date($1),$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13);
    }
}
