use rust_json_parser::parse;

fn main() {
    let inputs = [
        r#""The quick brown fox jumps over the lazy dog""#,
        "3.14159265358979",
        r#""missing end quote"#,
    ];

    for input in inputs {
        println!("Input: {input}");

        match parse(input) {
            Ok(value) => println!("Parsed: {value:?}"),
            Err(error) => println!("Error: {error}"),
        }
        println!();
    }
}
