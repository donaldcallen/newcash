function extract(line, regexp, result) {
    result = match(line, regexp, extracted);
    if (result) {
        return extracted[1];
    } else {
        print "extract: match failed";
        exit 1;
    }
}

BEGIN {
    print "m4_define(EPSILON, 0.0001)m4_dnl";
    print "m4_define(NEW_UUID,lower(hex(randomblob(16))))m4_dnl";
}
/ACCOUNT_FLAG/ || /SPLIT_FLAG/ || /COMMODITY_FLAG/ {
    value = extract($0, "=(.*);");
    name = extract($0, "const (.*):");
    printf "m4_define(%s,m4_eval(%s))m4_dnl\n", name, value;
    printf "m4_define(%s_BIT,%d)m4_dnl\n", name, substr(value,4,length(value)-3);
}



