mod iptc;
mod nl;

fn main() {
    println!("Hi there {} {}", unsafe { libc::geteuid() }, unsafe {
        libc::getuid()
    });
}
