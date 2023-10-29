use chrono::Local;
use colored::ColoredString;
/// A utility function that prepends the current date and time to a message.
///
/// # Arguments
///
/// * `msg` - The message to be logged.
///
/// # Returns
///
/// A formatted string with the current date and time prepended to the original message.
pub fn log_with_timestamp(msg: ColoredString) {
  let now = Local::now();
  println!("[{}] {}", now.format("%Y-%m-%d %H:%M:%S"), msg);
}

