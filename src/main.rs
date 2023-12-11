use syscalls::{Sysno, syscall};
use std::io::{self, BufRead};
use std::io::Write;
use std::ffi::{CString, c_char};
use std::env;
use std::path::Path;
use std::ptr;

const HELP_TEXT: &str = "arsh shell
copyright 2023 arrynfr

Available builtin commands:
cd          help";

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
            let c_argv = c_argv.iter().map(|arg| arg.as_ptr()).chain([ptr::null()]).collect::<Vec<*const c_char>>();

            let mut c_envp = Vec::new();
            for (key,val) in  env::vars() {
                let env_str = format!("{key}={val}");
                c_envp.push(CString::new(env_str).unwrap());
            }
            let c_envp = c_envp.iter().map(|arg| arg.as_ptr()).chain([ptr::null()]).collect::<Vec<*const c_char>>();

            match unsafe { syscall!(Sysno::execve, c_cmd.as_ptr(), c_argv.as_ptr(), c_envp.as_ptr()) } {
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

fn search_in_path(cmd: &str) -> String {
    if Path::new(cmd).exists() {
        return cmd.to_owned();
    }

    let test_cmd = format!("./{cmd}");
    if Path::new(&test_cmd).exists() {
        return test_cmd;
    } else {
        let env_path = env!("PATH").split(":");
        for p in env_path {
            let p = format!("{p}/{cmd}");
            if Path::new(&p).exists() {
                return p;
            }
        }
    }
    return cmd.to_owned();
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
			"exit"	=> {
					std::process::exit(0);
				   }
                        _       => {
                                        let cmd = search_in_path(cmd);
                                        execute_program(&cmd, argv)
                                   }
                    }
                }
                Err(err) => {eprintln!("Couldn't read line: {err}")}
            }
            print!("{} > ", get_cwd());
            io::stdout().flush().unwrap();
        }
    }
}
