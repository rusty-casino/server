use easy_api;

fn works() {
    println!("It works")
}

fn main() {
    easy_api::add_tap_api("/ping", works);

    easy_api::start();

    loop {
        easy_api::park();
    }
}
