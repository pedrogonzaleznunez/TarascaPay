# TarascaPay

TarascaPay is an open-source expense-splitting application designed to make group finances transparent, fast, and simple. Think of it as a privacy-focused, self-hostable alternative to Splitwise.


## Rules 
To maintain the absolute highest standards of code quality, deep architectural understanding, and maintainability, this project enforces a strict human-only development policy.

* **No AI-Generated Code:** The use of Copilot, ChatGPT, Claude, or any other LLM/AI code-generation tool is strictly prohibited.
* **Human Review Only:** All logic, structural decisions, and optimizations must be written and understood fully by the human author.
* **Handcrafted Rust:** Every line of Axum, Tokio, and database logic must be typed manually to ensure complete ownership over the codebase.

## Stack

* **Language:** Rust (Edition 2021)
* **Web Framework:** Axum 0.8
* **Async Runtime:** Tokio 1.x
* **Serialization:** Serde

# how to run

Make sure you have the latest stable Rust toolchain installed on your system:
```bash
rustup update stable
```

Run the development server:
```bash
cargo run
```