use std::io::{stdout, Write};
use std::env::args;
use std::path::Path;
use std::fmt::{self, Display, Formatter};

use crossterm::ExecutableCommand;
use crossterm::terminal::{
	EnterAlternateScreen,
	LeaveAlternateScreen,
	Clear,
	ClearType,
	enable_raw_mode,
	disable_raw_mode,
	size,
};
use crossterm::event::{self, Event, KeyEvent, KeyCode, KeyModifiers};
use crossterm::style::{style, Color, Print, PrintStyledContent};
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;

use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
struct Fragment {
	shared: usize,
	count: usize,
	data: Vec<char>,
}

#[derive(Copy, Clone)]
struct CharWriter<'a>(&'a [char]);

impl<'a> Display for CharWriter<'a> {
	fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
		for char in self.0 {
			char.fmt(f)?;
		}

		Ok(())
	}
}

fn load_fragments<P: AsRef<Path>>(path: P) -> crossterm::Result<Vec<Vec<char>>> {
	use std::fs::File;
	use std::io::{BufRead, BufReader};

	let file = BufReader::new(File::open(path)?);

	let mut out = Vec::new();
	for line in file.lines() {
		out.push(line?.chars().collect());
	}
	Ok(out)
}

fn into_vec_of_chars(x: &str) -> Vec<char> {
	x.chars().collect()
}

fn num_shared(buffer: &[char], fragment: &[char]) -> usize {
	for len in (0 ..= fragment.len().min(buffer.len())).rev() {
		if &fragment[0 .. len] == &buffer[buffer.len() - len ..] {
			return len;
		}
	}

	return 0;
}

fn main() -> crossterm::Result<()> {
	let mut args = args();

	args.next().expect("no executable name?");
	let filename = if let Some(filename) = args.next() {
		filename
	} else {
		println!("no fragments file provided?");
		return Ok(());
	};

	// initial setup

	let mut stdout = stdout();
	let (mut cols, mut rows) = size()?;

	let fragments = load_fragments(filename)?;
	let mut hash_map: HashMap<_, usize> = HashMap::new();
	for fragment in fragments {
		*hash_map.entry(fragment).or_insert(0) += 1;
	}
	let mut fragments = Vec::new();
	for (key, value) in hash_map {
		fragments.push(Fragment {
			data: key,
			count: value,
			shared: 0,
		});
	}

	stdout.execute(EnterAlternateScreen)?;
	enable_raw_mode()?;

	let mut edit_buffer = Vec::new();

	loop {
		// keyboard events

		match event::read()? {
			Event::Key(KeyEvent {code, modifiers}) => {
				if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
					break;
				}

				if let KeyCode::Char(ch) = code {
					edit_buffer.push(ch);
				}

				if code == KeyCode::Backspace {
					edit_buffer.pop();
				}
			},
			Event::Mouse(_) => {},
			Event::Resize(ncols, nrows) => {
				cols = ncols;
				rows = nrows;
			},
		}

		// logic

		for fragment in fragments.iter_mut() {
			fragment.shared = num_shared(&edit_buffer, &fragment.data);
		}

		fragments.sort_unstable_by_key(|fragment| (fragment.shared, fragment.count));

		let mut fragments_iter = fragments.iter().rev();

		let max_size = fragments.iter().map(|fragment| {
			fragment.data.len() - fragment.shared + 5 // space for " (xx)"
		}).max().unwrap_or(0);

		// rendering

		stdout.queue(Clear(ClearType::All))?;

		// todo: all of these variable names are terrible
		let max_col = (cols as usize).saturating_sub(max_size);
		let start = edit_buffer.len().saturating_sub(max_col);
		let cursor_col = max_col.min(edit_buffer.len());

		stdout.queue(MoveTo(0, 0))?;
		stdout.queue(Print(CharWriter(&edit_buffer[start ..])))?;

		for i in 1 .. rows {
			if let Some(fragment) = fragments_iter.next() {
				stdout.queue(MoveTo((cursor_col - fragment.shared) as u16, i))?;
				stdout.queue(Print(CharWriter(&fragment.data)))?;
				stdout.queue(Print(format!(" ({:02})", fragment.count.min(99))))?;
			} else {
				break;
			}
		}

		stdout.flush()?;
	}

	disable_raw_mode()?;
	stdout.execute(LeaveAlternateScreen)?;

	println!("{}", CharWriter(&edit_buffer));

	Ok(())
}
