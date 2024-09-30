use std::{env, fs::File, io::Read};

fn main() {
    let args: Vec<String> = env::args().collect();

    //check length of args is enough
    if args.len() < 2 {
        println!("No file name provided")
    }

    //open file and handle both success and error cases with match
    let mut file = match File::open(&args[1]) {
        Ok(file) => file,
        Err(err) => {
            println!("Error opening file: {}", err);
            return;
        }
    };

    let mut contents = String::new();

    //read file into contents and handle error case in if statement
    if let Err(err) = file.read_to_string(&mut contents) {
        println!("Error reading file: {}", err)
    }

    println!("{} total words", contents.split_whitespace().count());
}
