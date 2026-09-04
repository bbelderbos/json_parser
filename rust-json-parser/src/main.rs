use rust_json_parser::parse;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
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
        .ok_or("missing \"name\" field")?
        .as_str()
        .ok_or("\"name\" should be a string")?;
    let features = value
        .get("features")
        .ok_or("missing \"features\" field")?
        .as_array()
        .ok_or("\"features\" should be an array")?;
    let author = value
        .get("metadata")
        .ok_or("missing \"metadata\" field")?
        .get("author")
        .ok_or("missing \"author\" field")?
        .as_str()
        .ok_or("\"author\" should be a string")?;

    println!("name: {name}");
    println!("features: {features:?}");
    println!("author: {author}");
    println!();
    println!("{value}");

    Ok(())
}
