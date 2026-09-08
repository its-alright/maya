fn main() {
    // Указываем Cargo пересобирать проект при любом изменении в папке migrations
    println!("cargo:rerun-if-changed=migrations");
}