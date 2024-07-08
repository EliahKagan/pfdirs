//! Reports information from multiple sources about where `Program Files`
//! folders are located on a Windows system.

fn column_width(names: &[&str]) -> usize {
    names
        .iter()
        .map(|name| name.chars().count())
        .max()
        .unwrap_or(0)
}

fn report_environment_variables() {
    let names = [
        "ProgramFiles",
        "ProgramFiles(Arm)",
        "ProgramFiles(x86)",
        "ProgramW6432",
    ];

    let width = column_width(&names);

    println!("Relevant environment variables:");
    println!();

    for name in names {
        let path_item = std::env::var(name).unwrap_or_else(|_| "[variable does not exist]".into());
        println!("  {name:<width$}  {path_item}");
    }
}

fn main() {
    report_environment_variables();

    // FIXME: Do the other reports.
}
