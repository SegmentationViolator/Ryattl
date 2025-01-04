//    Copyright (C) 2024 Segmentation Violator <segmentationviolator@proton.me>

//    This program is free software: you can redistribute it and/or modify
//    it under the terms of the GNU General Public License as published by
//    the Free Software Foundation, either version 3 of the License, or
//    (at your option) any later version.

//    This program is distributed in the hope that it will be useful,
//    but WITHOUT ANY WARRANTY; without even the implied warranty of
//    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//    GNU General Public License for more details.

//    You should have received a copy of the GNU General Public License
//    along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    cmp, env, fmt, fs,
    io::{self, Write},
    path, process,
};

use clap::Parser;
use colored::Colorize;

mod parsing;

const TASKLIST_FILENAME: &str = ".ryattl";

/// Yet Another Terminal-based Task List written in Rust
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Add a task
    Add {
        /// Priority associated with the task ('min', 'max' or a whole number)
        #[arg(short, value_parser = parsing::parse_priority, default_value_t = Priority::Min)]
        priority: Priority,

        /// Message associated with the task
        task: String,
    },

    /// Initiate a new task list in the current directory
    Init,

    /// List all the tasks
    List,

    /// Modify a task
    Modify {
        /// Priority associated with the task ('min', 'max' or a whole number)
        #[arg(short, value_parser = parsing::parse_priority, group = "modifications")]
        priority: Option<Priority>,

        /// ID associated with the task
        #[arg(value_parser = parsing::parse_task_id, requires = "modifications")]
        task_id: usize,
    },

    /// Remove a task
    Remove {
        /// ID associated with the task
        #[arg(value_parser = parsing::parse_task_id)]
        task_id: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Priority {
    Max,
    Min,
    Value(usize),
}

struct Task {
    priority: Priority,
    message: String,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Value(n) => write!(f, "{}", n),
        }
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Max, Self::Max) | (Self::Min, Self::Min) => cmp::Ordering::Equal,
            (Self::Max, _) | (_, Self::Min) => cmp::Ordering::Greater,
            (_, Self::Max) | (Self::Min, _) => cmp::Ordering::Less,
            (Self::Value(x), Self::Value(y)) => x.cmp(y),
        }
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn main() -> process::ExitCode {
    match internal_main() {
        Ok(()) => process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{} {}", "error:".red().bold(), err);
            process::ExitCode::FAILURE
        }
    }
}

fn internal_main() -> Result<(), String> {
    let args = Args::parse();

    if let Command::Init = args.command {
        let tasklist_path = env::current_dir()
            .map_err(|err| err.to_string())?
            .join(TASKLIST_FILENAME);

        match tasklist_path.try_exists() {
            Ok(true) => {
                eprint!(
                    "{} this directory already has a tasklist\nDo you wish to overwrite it? {} ",
                    "warning:".yellow().bold(),
                    "[y/N]:".cyan().bold(),
                );

                io::stdout().flush().map_err(|err| err.to_string())?;

                let mut buffer = String::with_capacity(1);

                io::stdin()
                    .read_line(&mut buffer)
                    .map_err(|err| err.to_string())?;

                let Some('y') = buffer.chars().next() else {
                    return Ok(());
                };
            }
            Err(err) => return Err(err.to_string()),
            _ => (),
        }

        fs::File::create(tasklist_path).map_err(|err| err.to_string())?;

        println!(
            "{} a new tasklist in the current directory",
            "Initiated".green().bold()
        );

        return Ok(());
    }

    let tasklist_path = get_tasklist_path()?;
    let mut tasklist = get_tasklist(&tasklist_path)?;

    match args.command {
        Command::Add {
            priority,
            task: message,
        } => {
            'block: {
                let task = Task {
                    message: message.replace('\x1F', " "),
                    priority,
                };

                if priority == Priority::Max {
                    tasklist.push(task);
                    break 'block;
                }

                let mut begin = 0;
                let mut end = tasklist.len();

                while begin < end {
                    let pivot = (begin + end) / 2;
                    match tasklist[pivot].priority.cmp(&task.priority) {
                        cmp::Ordering::Less => begin = pivot + 1,
                        cmp::Ordering::Equal => break,
                        cmp::Ordering::Greater => end = pivot,
                    }
                }
                let pivot = (begin + end) / 2;

                if pivot == tasklist.len() {
                    tasklist.push(task);
                    break 'block;
                }

                tasklist.insert(pivot, task);
            }

            println!("{} a new task", "Added".green().bold());
        }

        Command::List => {
            if tasklist.is_empty() {
                println!("The task list is empty");
                return Ok(());
            }

            let mut buffer = String::new();

            for (index, task) in tasklist.iter().rev().enumerate() {
                buffer.push_str(&format!(
                    " {:^width$} | {}\n",
                    (index + 1).to_string().yellow(),
                    task.message.green(),
                    width = tasklist.len().ilog10() as usize + 1,
                ));
            }

            print!("{}", buffer);

            return Ok(());
        }

        Command::Modify { priority, task_id } => {
            let tasklist_len = tasklist.len();

            if task_id > tasklist_len {
                return Err(format!(
                    "invalid value '{}' for '{}': expected a value less than or equal to {}\n\nFor more information, try '{}'.",
                    task_id.to_string().yellow(),
                    "<TASK_ID>".bold(),
                    tasklist.len(),
                    "--help".bold(),
                ));
            }

            let task = unsafe { tasklist.get_unchecked_mut(tasklist_len - task_id) };
            let is_sorted = priority.is_none();

            if let Some(priority) = priority {
                task.priority = priority;
            }

            println!("{} the specified task", "Modified".green().bold());

            if !is_sorted {
                tasklist.sort_by_key(|task| task.priority);
                eprintln!(
                    "{} the priority was changed and as a result the task IDs might have also changed",
                    "warning:".yellow().bold(),
                );
            }
        }

        Command::Remove { task_id } => {
            let tasklist_len = tasklist.len();

            if task_id > tasklist_len {
                return Err("".to_owned());
            }

            tasklist.remove(tasklist_len - task_id);

            println!("{} the specified task", "Removed".green().bold());
        }

        _ => unreachable!(),
    }

    save_tasklist(tasklist_path, tasklist)
}

fn get_tasklist(tasklist_path: &path::Path) -> Result<Vec<Task>, String> {
    let tasklist: Result<Vec<Task>, _> = fs::read_to_string(tasklist_path)
        .map_err(|err| err.to_string())?
        .lines()
        .map(parsing::parse_task)
        .collect();

    tasklist.map(|mut tasklist| {
        tasklist.sort_by_key(|task| task.priority);
        tasklist
    })
}

fn get_tasklist_path() -> Result<path::PathBuf, String> {
    let mut tasklist_dir = env::current_dir().map_err(|err| err.to_string())?;

    loop {
        let tasklist_path = tasklist_dir.join(TASKLIST_FILENAME);

        if tasklist_path.exists() && tasklist_path.is_file() {
            break;
        }

        match tasklist_dir.parent() {
            Some(pathbuf) => tasklist_dir = pathbuf.to_owned(),
            None => return Err("this directory has no task list associated with it".to_owned()),
        }
    }

    Ok(tasklist_dir.join(TASKLIST_FILENAME))
}

fn save_tasklist(tasklist_path: path::PathBuf, tasklist: Vec<Task>) -> Result<(), String> {
    let mut buffer = String::new();

    for task in tasklist.into_iter() {
        buffer.push_str(&format!("{}\x1F{}\n", task.priority, task.message));
    }

    let mut tasklist_file = fs::File::create(tasklist_path).map_err(|err| err.to_string())?;

    tasklist_file
        .write_all(buffer.as_bytes())
        .map_err(|err| err.to_string())
}
