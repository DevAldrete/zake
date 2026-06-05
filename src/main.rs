use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};
use zake::app::AppState;
use zake::cli::{Cli, Command};
use zake::index::NoteIndex;
use zake::note;
use zake::notebook::Notebook;
use zake::search;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Init { path }) => {
            let notebook = Notebook::init(path)?;
            println!("Initialized Zake notebook at {}", notebook.root.display());
        }
        Some(Command::Doctor { path }) => {
            let notebook = Notebook::discover(path)?;
            let warnings = notebook.validate()?;
            let index = NoteIndex::build(&notebook);
            println!("Notebook: {}", notebook.root.display());
            println!("Notes: {}", index.notes.len());
            println!("Parse errors: {}", index.parse_errors.len());
            println!("Broken-link files: {}", index.broken_links.len());
            if warnings.is_empty() && index.parse_errors.is_empty() {
                println!("Doctor: ok");
            } else {
                for warning in warnings {
                    println!("Warning: {warning}");
                }
                for (path, error) in index.parse_errors {
                    println!("Parse error: {}: {error}", path.display());
                }
            }
        }
        Some(Command::New { title, path }) => {
            let notebook = Notebook::discover(path)?;
            let created = note::create_note(&notebook, &title)?;
            println!("{}", created.path.display());
        }
        Some(Command::Search { query, path }) => {
            let notebook = Notebook::discover(path)?;
            for hit in search::ripgrep(&notebook.root, &query)? {
                println!("{}:{}:{}", hit.path.display(), hit.line, hit.text);
            }
        }
        None => {
            let notebook = match Notebook::discover(".") {
                Ok(notebook) => notebook,
                Err(err) => prompt_init_current_dir(err.to_string())?,
            };
            zake::tui::run(AppState::load(notebook))?;
        }
    }

    Ok(())
}

fn prompt_init_current_dir(reason: String) -> Result<Notebook> {
    println!("{reason}");
    print!("Initialize a Zake notebook here? [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if answer.trim().eq_ignore_ascii_case("y") || answer.trim().eq_ignore_ascii_case("yes") {
        Notebook::init(".")
    } else {
        anyhow::bail!("no notebook selected")
    }
}
