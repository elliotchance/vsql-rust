# vsql-sys

Raw bindings to libvsql, the SQL/CLI library of
[vsql](https://github.com/elliotchance/vsql), a single file or in-memory SQL
database. libvsql is compiled into the crate from the vsql amalgamation, so
there's nothing else to install.

Use the [`vsql`](https://crates.io/crates/vsql) crate instead, unless you need
to call libvsql directly. `amalgamation/libvsql.h` documents each routine.

See [vsql-rust](https://github.com/elliotchance/vsql-rust) for the
requirements (including clang-cl on Windows).
