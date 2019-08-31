// Constants
pub const INVESTMENT_REPORT_SIZE: usize = 30000;
pub const DOCUMENT_HEADER: &str = "\\documentclass[legalpaper]{article}
\\usepackage[landscape]{geometry}
\\usepackage{longtable}
\\begin{document}
\\title{Financial Report}
\\author{Don Allen}
\\maketitle
";

pub const ASSETS_FOOTER: &str = "\\hline
";
pub const LIABILITIES_FOOTER: &str = "\\end{longtable}
";

pub const INCOME_FOOTER: &str = "\\hline
";
pub const EXPENSES_FOOTER: &str = "\\end{longtable}
";

pub const INVESTMENTS_HEADER: &str = "
\\newpage
\\section{Investments}
";

pub const OPEN_POSITIONS_SUBSECTION_HEADER: &str = "\\subsection{Open Positions}
\\begin{longtable} {|l|r|r|}
\\hline
\\endhead
\\hline
\\endfoot
Name & Shares & Current Value\\\\
\\hline
";

pub const OPEN_POSITIONS_QUOTES_SUBSECTION_HEADER: &str = "\\subsection{Open Position Most Recent \
                                                           Quotes}
\\begin{longtable} {|l|l|}
\\hline
\\endhead
\\hline
\\endfoot
Name & Most Recent Quote\\\\
\\hline
";

pub const OPEN_POSITIONS_SUBSECTION_FOOTER: &str = "\\end{longtable}
\\newpage
";

pub const CAPITAL_GAIN_SUBSECTION_HEADER: &str = "\\subsection{Capital Gain}
";
pub const TOTAL_CAPITAL_GAIN_SUBSECTION_HEADER: &str = "
\\subsection{Capital Gain + Dividends}
";
pub const ANNUALIZED_GAIN_SUBSECTION_HEADER: &str = "
\\subsection{Annualized Return from Capital Gain}
";
pub const TOTAL_ANNUALIZED_GAIN_SUBSECTION_HEADER: &str = "
\\subsection{Annualized Return from Capital Gain + Dividends}
";
pub const INVESTMENT_SUBSECTION_HEADER: &str = "\\begin{longtable} {|l|r|}
\\hline
\\endhead
\\hline
\\endfoot
";

pub const INVESTMENT_SUBSECTION_FOOTER: &str = "\\end{longtable}
\\newpage
";

pub const DOCUMENT_FOOTER: &str = "\\end{document}
";
