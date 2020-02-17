// Constants
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
