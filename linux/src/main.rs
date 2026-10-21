use shared::hello_core;

#[tokio::main]
async fn main() {
    println!("{}", hello_core());
}