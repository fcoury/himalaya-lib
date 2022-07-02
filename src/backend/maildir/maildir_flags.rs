// himalaya-lib, a Rust library for email management.
// Copyright (C) 2022  soywod <clement.douin@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::msg::Flags;

use super::maildir_flag;

pub fn from_maildir_entry(entry: &maildir::MailEntry) -> Flags {
    entry.flags().chars().map(maildir_flag::from_char).collect()
}

pub fn to_normalized_string(flags: &Flags) -> String {
    String::from_iter(flags.iter().filter_map(maildir_flag::to_normalized_char))
}