// #[cfg(test)]
// mod test {
//     use std::env;
//
//     use tmux_sessionizer::config_entry::{ConfigEntry, EntryKind};
//
//     #[test]
//     fn parses_plain_entry() {
//         let input = "e ~/test/file";
//
//         let result = ConfigEntry::parse_from_string(input).unwrap();
//
//         assert_eq!(
//             result,
//             EntryKind::Plain(ConfigEntry::new(
//                 "file".to_string(),
//                 format!("{}/test/file", env::var("HOME").unwrap()),
//             ))
//         );
//     }
//
//     #[test]
//     fn parses_dir_entry() {
//         let input = "d ~/test/file";
//
//         let result = ConfigEntry::parse_from_string(input).unwrap();
//
//         assert_eq!(
//             result,
//             EntryKind::Dir(format!("{}/test/file", env::var("HOME").unwrap()))
//         );
//     }
//
//     #[test]
//     fn does_envsubst_tilde() {
//         let input = "e ~/test/file";
//
//         let result = ConfigEntry::parse_from_string(input).unwrap();
//
//         assert_eq!(
//             result,
//             EntryKind::Plain(ConfigEntry::new(
//                 "file".to_string(),
//                 format!("{}/test/file", env::var("HOME").unwrap()),
//             ))
//         );
//     }
//
//     #[test]
//     fn does_envsubst_home() {
//         let input = "e $HOME/test/file";
//
//         let result = ConfigEntry::parse_from_string(input).unwrap();
//
//         assert_eq!(
//             result,
//             EntryKind::Plain(ConfigEntry::new(
//                 "file".to_string(),
//                 format!("{}/test/file", env::var("HOME").unwrap()),
//             ))
//         );
//     }
// }
