fn main() {
    let age = 20;
    let can_vote = age >= 18;

    println!("If you are {} y/o, you {} vote",age,if can_vote{"can"} else {"can not"});

    let is_logged_in = true;

    if is_logged_in {
        println!("Welcome back!");
    } else {
        println!("Please log in.");
    }

    // COOL BUT NO SO COOL SINTAX
    let time = 20;
    let greeting = if time < 18 {
        "Good day."
    } else {
        "Good evening."
    };
    println!("{}", greeting);

    // DO NOT MIX TYPES --> THIS WILL FAIL
    // let number = 5;
    // let result = if number < 10 { "Too small" } else { 100 };
    // println!("{}", result);

    let day = 4;

    match day {
        1 => println!("Monday"),
        2 => println!("Tuesday"),
        3 => println!("Wednesday"),
        4 => println!("Thursday"),
        5 => println!("Friday"),
        6 => println!("Saturday"),
        7 => println!("Sunday"),
        _ => println!("Invalid day."),
    }

}