#!/bin/dash

cd cambridge_trust_importer/
cargo fmt
cd ..
cd composite_register/
cargo fmt
cd ..
cd newcash/
cargo fmt
cd ..
cd report_generator/balance_sheet_income_expense_statement
cargo fmt
cd ../..
cd report_generator/investments
cargo fmt
cd ../..
cd rust_library/
cargo fmt
cd ..
cd vanguard_importer/
cargo fmt
cd ..
cd verifier/
cargo fmt
cd ..
