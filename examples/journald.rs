#[cfg(unix)]
fn main() {
    use logforth::append::Journald;

    let append = Journald::new().unwrap();
    logforth::builder().dispatch(|d| d.append(append)).apply();

    log::error!("Hello, journald at ERROR!");
    log::warn!("Hello, journald at WARN!");
    log::info!("Hello, journald at INFO!");
    log::debug!("Hello, journald at DEBUG!");
    log::trace!("Hello, journald at TRACE!");
}

#[cfg(not(unix))]
fn main() {
    println!("This example is only for Unix-like systems.");
}
