fn main() {

    // unmutable variables with its types
    let my_num:i32 = 5;         // integer
    let my_double:f64 = 5.99;   // float
    let my_letter:char = 'D';    // character
    let my_bool :bool= true;     // boolean
    let my_text:&str = "Hello";  // string

    const BIRTHYEAR: i32 = 2002;

    // Constants must always declare the type
    const MINUTES_PER_HOUR: i32 = 60;

    // ERROR
    const MINUTES_PER_HOUR= 60;

}