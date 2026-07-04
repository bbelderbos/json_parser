use rust_json_parser::parse_json;

fn main() {
    let inputs = [
        r#""The quick brown fox jumps over the lazy dog""#,
        "3.14159265358979",
        r#""missing end quote"#,
    ];

    for input in inputs {
        println!("Input: {input}");
        match parse_json(input) {
            Ok(value) => println!("Parsed: {value:?}"),
            Err(error) => println!("Error: {error}"),
        }
        println!();
    }
}
