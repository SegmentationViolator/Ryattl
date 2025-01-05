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

use std::num;

use crate::{Priority, Task};

pub const RECORD_SEPERATOR: char = '\n';
pub const UNIT_SEPERATOR: char = '\x1F';

pub fn parse_priority(string: &str) -> Result<Priority, String> {
    match string.trim() {
        "max" => Ok(Priority::Max),
        "min" => Ok(Priority::Min),
        _ => string
            .parse()
            .map(Priority::Value)
            .map_err(|err| match err.kind() {
                num::IntErrorKind::PosOverflow => {
                    "the number is too big, you might want to use 'max' instead".to_owned()
                }
                _ => "expected 'min', 'max' or a whole number".to_owned(),
            }),
    }
}

pub fn parse_task(string: &str) -> Result<Task, String> {
    let mut items = string.splitn(2, UNIT_SEPERATOR);

    let Some(priority) = items.next().and_then(|string| parse_priority(string).ok()) else {
        return Err("the task list file is corrupted".to_owned());
    };

    let Some(message) = items.next() else {
        return Err("the task list file is corrupted".to_owned());
    };

    Ok(Task {
        priority,
        message: message.to_owned(),
    })
}

pub fn parse_task_id(string: &str) -> Result<usize, String> {
    string
        .trim()
        .parse::<num::NonZeroUsize>()
        .map(|n| n.get())
        .map_err(|err| match err.kind() {
            num::IntErrorKind::PosOverflow => "the number is too big to be a valid ID".to_owned(),
            _ => "expected a non-zero whole number".to_owned(),
        })
}
