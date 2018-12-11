# hdbconnect

A pure rust SQL driver for SAP HANA(TM).

## Usage

Add hdbconnect to the dependencies section in your project's `Cargo.toml`, with

```toml
[dependencies]
hdbconnect = "0.11"
```

and add this to your crate root:

```rust
extern crate hdbconnect;
```

Assume you have a HANA (e.g. a HANA Express) accessible at port 30333 on host `hxehost`,
and you can log on to it as user `HORST` with password `SECRET`.

Then a first simple test might look like this:

```rust
use hdbconnect::{Connection, HdbResult, IntoConnectParams};

pub fn main() -> HdbResult<()> {
    // Get a connection
    let params = "hdbsql://HORST:SECRET@hxehost:39013".into_connect_params()?;
    let mut connection = Connection::new(params)?;

    // Cleanup if necessary, and set up a test table
    connection.multiple_statements_ignore_err(vec!["drop table FOO_SQUARE"]);
    connection.multiple_statements(vec![
        "create table FOO_SQUARE ( f1 INT primary key, f2 INT)",
    ])?;

    // Insert some test data
    let mut insert_stmt = connection.prepare("insert into FOO_SQUARE (f1, f2) values(?,?)")?;
    for i in 0..100 {
        insert_stmt.add_batch(&(i, i * i))?;
    }
    insert_stmt.execute_batch()?;

    // Read the table data directly into an adequate rust data structure!
    let stmt = "select * from FOO_SQUARE order by f1 asc";
    let result: Vec<(usize, usize)> = connection.query(stmt)?.try_into()?;

    // Verify ...
    for (idx, (n, square)) in result.into_iter().enumerate() {
        assert_eq!(idx, n);
        assert_eq!(idx * idx, square);
    }
    Ok(())
}
```

## Documentation

See <https://docs.rs/hdbconnect/> for the full functionality of hdbconnect.

There you also find more code examples, e.g. in the description of module `code_examples`.

## Crate Features

### `tls`

The `tls` feature adds the capability to use TLS in the communication to HANA, and adds dependencies to `rustls` and `webpki`.

See [ConnectParams](https://docs.rs/hdbconnect/*/hdbconnect/struct.ConnectParams.html)
for how to use hdbconnect with tls.

See [HANA in SCP](HANA_in_SCP.md) for instructions how to obtain the necessary server certificate from a HANA in SAP Cloud Platform.

## Versions

See the [change log](https://github.com/emabee/rust-hdbconnect/blob/master/CHANGELOG.md).
