extern crate libc;

use std::alloc::System;
use std::io::{self, Write};
use std::ffi::CString;
use std::env;

#[global_allocator]
static A: System = System;

fn set_variable(name: &str, value: &str) {
    env::set_var(name, value);
}

fn get_variable(name: &str) -> Option<String> {
    env::var(name).ok()
}

fn expand_variables(command: &str) -> String {
    let mut expanded_command = String::new();
    let mut iter = command.chars().peekable();

    while let Some(ch) = iter.next() {
        if ch == '$' {
            if let Some(next_ch) = iter.next() {
                if next_ch == '(' {
                    let mut variable_name = String::new();
                    while let Some(inner_ch) = iter.next() {
                        if inner_ch == ')' {
                            break;
                        }
                        variable_name.push(inner_ch);
                    }
                    if let Some(value) = get_variable(&variable_name) {
                        expanded_command.push_str(&value);
                    } else {
                        eprintln!("{}Error: Variable '{}' not found{}", RED, variable_name, RESET);
                    }
                    continue;
                }
            }
            expanded_command.push(ch);
        } else {
            expanded_command.push(ch);
        }
    }

    expanded_command
}

// ANSI escape codes
const RED: &str = "\x1b[31m";
const _GREEN: &str = "\x1b[32m";
const _YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

fn change_directory(dir: &str) -> Result<(), String> {
    unsafe {
        let c_dir = CString::new(dir).expect("CString::new failed for directory");
        if libc::chdir(c_dir.as_ptr()) == 0 {
            Ok(())
        } else {
            Err("Failed to change directory".to_string())
        }
    }
}

// Ctrl+C (SIGINT)
extern "C" fn handle_sigint(_signo: libc::c_int) {
    print!("\n{} > ", env::current_dir().unwrap().to_string_lossy());
    io::stdout().flush().unwrap();
}

fn setup_signal_handler() {
    unsafe {
        libc::signal(libc::SIGINT, handle_sigint as libc::sighandler_t);
    }
}

fn main() {
    setup_signal_handler();
    loop {
        let current_dir = env::current_dir().unwrap();
        let current_dir_str = current_dir.to_string_lossy();

        print!("{} > ", current_dir_str);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).unwrap() == 0 {
            println!("\nexit");
            break;
        };

        let input = input.trim();
        let input_expanded = expand_variables(input);

        match input {
            _ if input.starts_with("cd ") => {
                match change_directory(&input[3..]) {
                    Ok(_) => (),
                    Err(error) => eprintln!("Error: {}", error),
                }
            }
            _ if input_expanded.starts_with("set ") => {
                let parts: Vec<&str> = input_expanded.splitn(3, ' ').collect();
                if parts.len() == 3 {
                    set_variable(parts[1], parts[2]);
                } else {
                    eprintln!("{}Error: Invalid set command{}", RED, RESET);
                }
            }
            "exit" | "quit" => {
                println!("exit");
                break;
            }
            _ => {
                if !input_expanded.is_empty() {
                    match execute_command(&input_expanded) {
                        Ok(output) => {
                            let _ = output;
                        }
                        Err(error) => eprintln!("{}Error: {}{}", RED, error, RESET),
                    }
                }
            }
        }
    }
}

fn execute_command(command: &str) -> Result<String, String> {
    // Split the command into arguments
    let args: Vec<&str> = command.split_whitespace().collect();

    // Use fork and exec to run the command
    unsafe {
        let child_pid = libc::fork();

        match child_pid {
            -1 => {
                // Fork failed
                Err("Fork failed".to_string())
            }
            0 => {
                // Child process
                let c_command = CString::new(args[0]).expect("CString::new failed for command");
                let c_args: Vec<CString> = args.iter()
                    .map(|&arg| CString::new(arg).expect("CString::new failed for argument"))
                    .collect();
                let c_args_ptrs: Vec<*const libc::c_char> = c_args.iter().map(|cstr| cstr.as_ptr()).collect();

                // Add a null pointer at the end of array as per man execve
                let mut c_args_ptrs_with_null = c_args_ptrs.clone();
                c_args_ptrs_with_null.push(std::ptr::null());

                libc::execvp(c_command.as_ptr(), c_args_ptrs_with_null.as_ptr());
                
                // execvp only returns if an error occurs
                eprintln!("{}c_shell: {:?}: {:?}{}", RED, c_command, CString::from_raw(libc::strerror(*libc::__errno_location())), RESET);
                libc::exit(1);
            }
            _ => {
                // Parent process
                let mut status: libc::c_int = 0;
                libc::waitpid(child_pid, &mut status, 0);

                if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0 {
                    // Child process exited successfully
                    Ok("Command executed successfully".to_string())
                } else {
                    // Child process encountered an error
                    Err("Command execution failed".to_string())
                }
            }
        }
    }
}
