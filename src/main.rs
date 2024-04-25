use super::*;

pub fn main() {
    env_logger::init();
    ctrlc::set_handler(move || {
      if SHUTTING_DOWN.fetch_or(true, atomic::Ordering::Relaxed) {
        process::exit(1);
      }
  
      eprintln!("Shutting down gracefully. Press <CTRL-C> again to shutdown immediately.");
  
      LISTENERS
        .lock()
        .unwrap()
        .iter()
        .for_each(|handle| handle.graceful_shutdown(Some(Duration::from_millis(100))));
  
      gracefully_shutdown_indexer();
    })
    .expect("Error setting <CTRL-C> handler");
  
    let args = Arguments::parse();
  
    let minify = args.options.minify;
  
    match args.run() {
      Err(err) => {
        eprintln!("error: {err}");
        err
          .chain()
          .skip(1)
          .for_each(|cause| eprintln!("because: {cause}"));
        if env::var_os("RUST_BACKTRACE")
          .map(|val| val == "1")
          .unwrap_or_default()
        {
          eprintln!("{}", err.backtrace());
        }
  
        gracefully_shutdown_indexer();
  
        process::exit(1);
      }
      Ok(output) => {
        if let Some(output) = output {
          output.print_json(minify);
        }
        gracefully_shutdown_indexer();
      }
    }
  }
  
