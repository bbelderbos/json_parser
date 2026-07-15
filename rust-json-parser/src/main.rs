use rust_json_parser::JsonParser;
use rust_json_parser::JsonValue;
use rust_json_parser::Result;

fn main() {
    let inputs = [
        r#""The quick brown fox jumps over the lazy dog""#,
        "3.14159265358979",
        r#""missing end quote"#,
    ];

    fn parse_json(input: &str) -> Result<JsonValue> {
        let mut parser = JsonParser::new(input)?;
        parser.parse()
    }

    for input in inputs {
        println!("Input: {input}");

        match parse_json(input) {
            Ok(value) => println!("Parsed: {value:?}"),
            Err(error) => println!("Error: {error}"),
        }
        println!();
    }
}
