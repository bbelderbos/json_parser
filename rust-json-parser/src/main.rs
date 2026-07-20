use rust_json_parser::{parse, Result};

fn main() -> Result<()> {
    let json = r#"{
        "name": "Rust JSON Parser",
        "version": 1.0,
        "features": ["arrays", "objects", "nesting"],
        "metadata": {
            "author": "You",
            "complete": true
        }
    }"#;

    let value = parse(json)?;

    let name = value
        .get("name")
        .expect("missing \"name\" field")
        .as_str()
        .expect("\"name\" should be a string");
    let features = value
        .get("features")
        .expect("missing \"features\" field")
        .as_array()
        .expect("\"features\" should be an array");
    let author = value
        .get("metadata")
        .expect("missing \"metadata\" field")
        .get("author")
        .expect("missing \"author\" field");

    println!("name: {name}");
    println!("features: {features:?}");
    println!("author: {author}");
    println!();
    println!("{value}");

    Ok(())
}
