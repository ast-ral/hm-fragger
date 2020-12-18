use std::io::{stdout, Write};

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
use crossterm::Result;
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

fn load_fragments() -> Result<Vec<Fragment>> {
	use std::fs::File;
	use std::io::{BufRead, BufReader};

	let file = BufReader::new(File::open("fragments.txt")?);

	let mut out = Vec::new();
	for line in file.lines() {
		out.push(Fragment {
			shared: 0,
			count: 0,
			data: line?.chars().collect(),
		});
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

fn main() -> Result<()> {
	//let x = "abcdefghijklmnop";
	//let y = "lmnophuteons";
	//let x = into_vec_of_chars(x);
	//let y = into_vec_of_chars(y);
	//dbg!(num_shared(&x, &y));

	let mut stdout = stdout();
	let (mut cols, mut rows) = size()?;

	let fragments = load_fragments()?;
	let mut hash_map: HashMap<_, usize> = HashMap::new();
	for fragment in fragments {
		*hash_map.entry(fragment).or_insert(0) += 1;
	}
	let mut fragments = Vec::new();
	for (mut key, value) in hash_map {
		key.count = value;
		fragments.push(key);
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
		// todo: get rid of this allocation
		let string: String = (&edit_buffer[start ..]).iter().collect();
		stdout.queue(MoveTo(0, 0))?;
		stdout.queue(Print(string))?;

		for i in 1 .. rows {
			if let Some(fragment) = fragments_iter.next() {
				stdout.queue(MoveTo((cursor_col - fragment.shared) as u16, i))?;
				// todo: get rid of this allocation
				let count = fragment.count;
				let mut fragment: String = fragment.data.iter().collect();
				fragment += &format!(" ({:02})", count.min(99));
				stdout.queue(Print(fragment))?;
			} else {
				break;
			}
		}

		stdout.flush()?;
	}

	disable_raw_mode()?;
	stdout.execute(LeaveAlternateScreen)?;

	// todo: get rid of this allocation
	println!("{}", edit_buffer.iter().collect::<String>());

	Ok(())
}
