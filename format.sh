#!/bin/dash

cd cambridge_trust_importer/
cargo +nightly fmt
cd ..
cd composite_register/
cargo +nightly fmt
cd ..
cd newcash/
cargo +nightly fmt
cd ..
cd report_generator/
cargo +nightly fmt
cd ..
cd rust_library/
cargo +nightly fmt
cd ..
cd vanguard_importer/
cargo +nightly fmt
cd ..
cd verifier/
cargo +nightly fmt
cd ..
