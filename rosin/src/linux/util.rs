pub(crate) fn panic_and_print(msg: String) -> ! {
    println!("{}", msg);
    std::process::abort()
}