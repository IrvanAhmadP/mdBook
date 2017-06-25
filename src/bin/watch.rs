extern crate notify;
extern crate time;
extern crate crossbeam;

use std::path::Path;
use std::error::Error;

use self::notify::Watcher;
use std::time::Duration;
use std::sync::mpsc::channel;
use clap::ArgMatches;
use mdbook::MDBook;

use {get_book_dir, open};

// Watch command implementation
pub fn watch(args: &ArgMatches) -> Result<(), Box<Error>> {
    let book_dir = get_book_dir(args);
    let book = MDBook::new(&book_dir).read_config()?;

    let mut book = match args.value_of("dest-dir") {
        Some(dest_dir) => book.with_destination(dest_dir),
        None => book,
    };

    if args.is_present("curly-quotes") {
        book = book.with_curly_quotes(true);
    }

    if args.is_present("open") {
        book.build()?;
        if let Some(d) = book.get_destination() {
            open(d.join("index.html"));
        }
    }

    trigger_on_change(&mut book, |path, book| {
        println!("File changed: {:?}\nBuilding book...\n", path);
        if let Err(e) = book.build() {
            println!("Error while building: {:?}", e);
        }
        println!("");
    });

    Ok(())
}

// Calls the closure when a book source file is changed. This is blocking!
pub fn trigger_on_change<F>(book: &mut MDBook, closure: F) -> ()
    where F: Fn(&Path, &mut MDBook) -> ()
{
    use self::notify::RecursiveMode::*;
    use self::notify::DebouncedEvent::*;

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    let mut watcher = match notify::watcher(tx, Duration::from_secs(1)) {
        Ok(w) => w,
        Err(e) => {
            println!("Error while trying to watch the files:\n\n\t{:?}", e);
            ::std::process::exit(0);
        },
    };

    // Add the source directory to the watcher
    if let Err(e) = watcher.watch(book.get_source(), Recursive) {
        println!("Error while watching {:?}:\n    {:?}", book.get_source(), e);
        ::std::process::exit(0);
    };

    // Add the theme directory to the watcher
    if let Some(t) = book.get_theme_path() {
        watcher.watch(t, Recursive).unwrap_or_default();
    }


    // Add the book.{json,toml} file to the watcher if it exists, because it's not
    // located in the source directory
    if watcher
           .watch(book.get_root().join("book.json"), NonRecursive)
           .is_err() {
        // do nothing if book.json is not found
    }
    if watcher
           .watch(book.get_root().join("book.toml"), NonRecursive)
           .is_err() {
        // do nothing if book.toml is not found
    }

    println!("\nListening for changes...\n");

    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    NoticeWrite(path) |
                    NoticeRemove(path) |
                    Create(path) |
                    Write(path) |
                    Remove(path) |
                    Rename(_, path) => {
                        closure(&path, book);
                    },
                    _ => {},
                }
            },
            Err(e) => {
                println!("An error occured: {:?}", e);
            },
        }
    }
}
