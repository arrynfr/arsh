use syscalls::{Sysno, syscall};
use std::io::{self, BufRead};
use std::io::Write;
use std::ffi::{CString, c_char};

const HELP_TEXT: &str = "arsh shell
copyright 2023 arrynfr

Available builtin commands:
cd          help";

const PATH: [&str;2] = ["/usr/bin","/bin"];

fn change_directory(path: String) {
    let c_path: CString = CString::new(path.as_str()).unwrap();
    match unsafe { syscall!(Sysno::chdir, c_path.as_ptr()) } {
        Err(ret) => {eprintln!("{}", ret);}
        _ => {}
    }
}

fn get_cwd() -> String {
    let current_directory = vec![0; 256];
    unsafe {syscall!(Sysno::getcwd, current_directory.as_ptr(), current_directory.capacity())
            .expect("Couldn't get cwd");
    }
    String::from_utf8(current_directory).expect("Couldn't turn cwd into String")
}

fn execute_program(cmd: &str, argv: Vec::<&str>) {
    match unsafe { syscall!(Sysno::fork) } {
        Ok(0) => {
            // Child process
            let c_cmd: CString = CString::new(cmd).unwrap();
            let mut c_argv = Vec::new();
            for arg in argv {
                c_argv.push(CString::new(arg).unwrap());
            }
            let c_args = c_argv.iter().map(|arg| arg.as_ptr()).collect::<Vec<*const c_char>>();
            
            match unsafe { syscall!(Sysno::execve, c_cmd.as_ptr(), c_args.as_ptr(), 0) } {
                Ok(_none) => {unreachable!()}
                Err(err) => {
                    eprintln!("arsh: {} ({})", err.description()
                                                    .expect("No desciption provided"),
                                                    err.name().expect("No name provided"));
                    unsafe{ let _ = syscall!(Sysno::exit, err.into_raw());}
                }
            }
        }
        Ok(pid) => {
            // Parent process
            match unsafe { syscall!(Sysno::wait4, pid, 0, 0, 0) } {
                Err(err) => {eprintln!("{}", err);}
                _ => {}
            }
        }
        Err(err) => {
            eprintln!("fork() failed: {}", err);
        }
    }
}

fn main() {
    loop {
        print!("{} > ", get_cwd());
        io::stdout().flush().unwrap();
        for line in io::stdin().lock().lines() {
            match {line} {
                Ok(cmd) => {
                    let mut args = cmd.split_whitespace();
                    let mut argv = Vec::new();
                    for arg in args.to_owned() {
                        argv.push(arg);
                    }
                    let cmd = args.next().unwrap_or("");
                    match cmd {
                        ""      => {}
                        "help"  => {println!("{}", HELP_TEXT)}
                        "cd"    => {    if args.clone().count() <= 1 {
                                            change_directory(args.next().unwrap_or("").to_owned())
                                        } else {eprintln!("arsh: cd: too many arguments")}
                                    }
                        _       => {execute_program(cmd, argv)}
                    }
                }
                Err(err) => {eprintln!("Couldn't read line: {}", err)}
            }
            print!("{} > ", get_cwd());
            io::stdout().flush().unwrap();
        }
    }
}