fn main() {

    // this will repeat forever
    // loop {
    //     println!("This will repeat forever!");
    // }

    let mut count = 1;

    loop {
        println!("Hello World!");

        if count == 3 {
            break;
        }

        count += 1;
    }

    let mut count = 1;

    // result of a loop in a variable
    let result = loop {
        println!("Hello!");

        if count == 3 {
            break count; // Stop the loop and return the number 3
        }

        count += 1;
    };

    println!("The loop stopped at: {}", result);

}