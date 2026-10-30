// A complete program that uses vsql:
//
//   cargo run --example main

use vsql::{Connection, Date, Value};

fn main() -> vsql::Result<()> {
    let mut conn = Connection::open_in_memory()?;
    conn.execute(
        "CREATE TABLE products (
            id INTEGER NOT NULL,
            name VARCHAR(50) NOT NULL,
            price NUMERIC(10, 2),
            added DATE)",
        (),
    )?;

    let tx = conn.transaction()?;
    {
        let mut insert =
            tx.prepare("INSERT INTO products (id, name, price, added) VALUES (?, ?, ?, ?)")?;
        let date = |month, day| Date {
            year: 2024,
            month,
            day,
        };
        insert.execute((1, "Bolt", Value::Numeric("0.25".into()), date(1, 2)))?;
        insert.execute((2, "Nut", Value::Numeric("0.10".into()), date(3, 4)))?;
        insert.execute((3, "Washer", None::<f64>, None::<Date>))?;
    }
    tx.commit()?;

    let mut stmt =
        conn.prepare("SELECT name, price, added FROM products WHERE id > ? ORDER BY id")?;
    let mut rows = stmt.query([1])?;
    println!("{:?}", rows.column_names());
    for row in &mut rows {
        let row = row?;
        let name: String = row.get(0)?;
        let price: Option<String> = row.get(1)?;
        let added: Option<Date> = row.get(2)?;
        println!(
            "{name} {} {}",
            price.as_deref().unwrap_or("NULL"),
            added.map_or("NULL".to_string(), |d| d.to_string())
        );
    }

    if let Err(e) = conn.execute("SELECT * FROM nope", ()) {
        println!("{}: {e}", e.sqlstate().unwrap_or(""));
    }

    Ok(())
}
