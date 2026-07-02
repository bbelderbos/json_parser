use rust_json_parser::tokenizer::tokenize;

fn main() {
    let input = r#"{"name": "Alice", "age": 30}"#;
    let tokens = tokenize(input);
    println!("Input JSON: {input}");
    println!("\nTokens:");
    println!("{:?}", tokens);
}
